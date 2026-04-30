use crate::firewall_agent::{send_request, FirewallRequest};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use ipnet::IpNet;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct SmtpSecurityConfig {
    pub enabled: bool,
    pub event_log_path: PathBuf,
    pub report_path: PathBuf,
    pub auth_login_threshold_per_hour: usize,
    pub max_connections_per_ip_per_minute: usize,
    pub max_connections_per_ip_per_hour: usize,
    pub max_auth_attempts_per_ip_per_day: usize,
    pub temporary_block_enabled: bool,
    pub blocking_backend: FirewallBackend,
    pub firewall_agent_socket_path: PathBuf,
    pub malicious_block_duration: String,
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirewallBackend {
    Disabled,
    Nftables,
    Iptables,
    Fail2banLog,
    HostAgent,
}

impl Default for SmtpSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            event_log_path: PathBuf::from("data/security/smtp-security-events.jsonl"),
            report_path: PathBuf::from("data/security/security-summary.jsonl"),
            auth_login_threshold_per_hour: 5,
            max_connections_per_ip_per_minute: 10,
            max_connections_per_ip_per_hour: 100,
            max_auth_attempts_per_ip_per_day: 5,
            temporary_block_enabled: false,
            blocking_backend: FirewallBackend::Disabled,
            firewall_agent_socket_path: PathBuf::from("/run/ai-mail-butler/firewall-agent.sock"),
            malicious_block_duration: "1h".to_string(),
            whitelist: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "10.0.0.0/8".to_string(),
                "172.16.0.0/12".to_string(),
                "192.168.0.0/16".to_string(),
            ],
        }
    }
}

impl SmtpSecurityConfig {
    pub async fn load() -> Self {
        let path = std::env::var("SMTP_SECURITY_CONFIG")
            .unwrap_or_else(|_| "config/smtp-security-agent.yaml".to_string());
        let mut config = Self::default();

        if let Ok(raw) = fs::read_to_string(&path).await {
            config.apply_yaml_subset(&raw);
            info!("Loaded SMTP security config from {}", path);
        }

        if let Ok(value) = std::env::var("SMTP_SECURITY_ENABLED") {
            config.enabled = parse_bool(&value);
        }
        if let Ok(value) = std::env::var("SMTP_SECURITY_BLOCKING_BACKEND") {
            config.blocking_backend = FirewallBackend::parse(&value);
        }
        if let Ok(value) = std::env::var("SMTP_SECURITY_TEMP_BLOCK_ENABLED") {
            config.temporary_block_enabled = parse_bool(&value);
        }
        if let Ok(value) = std::env::var("SMTP_FIREWALL_AGENT_SOCKET") {
            config.firewall_agent_socket_path = PathBuf::from(value);
        }

        config
    }

    fn apply_yaml_subset(&mut self, raw: &str) {
        let mut section = String::new();
        let mut in_whitelist = false;

        for original in raw.lines() {
            let without_comment = original.split('#').next().unwrap_or("").trim_end();
            if without_comment.trim().is_empty() {
                continue;
            }

            let trimmed = without_comment.trim();
            if !without_comment.starts_with(' ') && trimmed.ends_with(':') {
                section = trimmed.trim_end_matches(':').to_string();
                in_whitelist = section == "whitelist";
                continue;
            }

            if in_whitelist && trimmed.starts_with('-') {
                let item = trimmed.trim_start_matches('-').trim();
                if !item.is_empty() {
                    self.whitelist.push(unquote(item));
                }
                continue;
            }

            if let Some((key, value)) = trimmed.split_once(':') {
                let key = key.trim();
                let value = unquote(value.trim());
                match (section.as_str(), key) {
                    ("smtp_security_agent", "enabled") => self.enabled = parse_bool(&value),
                    ("reporting", "report_path") => self.report_path = PathBuf::from(value),
                    ("thresholds", "malicious_score") => {
                        if let Ok(v) = value.parse::<usize>() {
                            self.auth_login_threshold_per_hour = v.max(30) / 20;
                        }
                    }
                    ("rate_limit", "max_connections_per_ip_per_minute") => {
                        if let Ok(v) = value.parse() {
                            self.max_connections_per_ip_per_minute = v;
                        }
                    }
                    ("rate_limit", "max_connections_per_ip_per_hour") => {
                        if let Ok(v) = value.parse() {
                            self.max_connections_per_ip_per_hour = v;
                        }
                    }
                    ("rate_limit", "max_auth_attempts_per_ip_per_day") => {
                        if let Ok(v) = value.parse() {
                            self.max_auth_attempts_per_ip_per_day = v;
                            self.auth_login_threshold_per_hour = v;
                        }
                    }
                    ("blocking", "backend") => {
                        self.blocking_backend = FirewallBackend::parse(&value);
                    }
                    ("blocking", "firewall_agent_socket_path") => {
                        self.firewall_agent_socket_path = PathBuf::from(value);
                    }
                    ("blocking", "temporary_block_enabled") => {
                        self.temporary_block_enabled = parse_bool(&value);
                    }
                    ("blocking", "malicious_block_duration") => {
                        self.malicious_block_duration = value;
                    }
                    ("logging", "event_log_path") => {
                        self.event_log_path = PathBuf::from(value);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl FirewallBackend {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "nft" | "nftables" => Self::Nftables,
            "iptables" => Self::Iptables,
            "fail2ban" | "fail2ban-log" | "fail2ban_compatible_log" => Self::Fail2banLog,
            "host-agent" | "host_agent" | "socket" | "unix-socket" => Self::HostAgent,
            _ => Self::Disabled,
        }
    }
}

#[derive(Debug, Clone)]
struct SourceState {
    connections: VecDeque<DateTime<Utc>>,
    auth_attempts: VecDeque<DateTime<Utc>>,
    suspicious_commands: VecDeque<DateTime<Utc>>,
    blocked_until: Option<DateTime<Utc>>,
    risk_score: i32,
}

impl Default for SourceState {
    fn default() -> Self {
        Self {
            connections: VecDeque::new(),
            auth_attempts: VecDeque::new(),
            suspicious_commands: VecDeque::new(),
            blocked_until: None,
            risk_score: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SmtpSecurityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub source_ip: String,
    pub source_port: u16,
    pub reverse_dns: Option<String>,
    pub risk_score: i32,
    pub risk_level: String,
    pub reason: String,
    pub action: String,
    pub block_duration: Option<String>,
    pub command: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SecurityDecision {
    Allow,
    Reject421(String),
}

#[derive(Debug, Clone, Serialize)]
struct SecuritySummary {
    timestamp: DateTime<Utc>,
    event_type: String,
    window: String,
    suspicious_sessions: usize,
    auth_probe_occurrences: usize,
    unique_source_ips: usize,
    top_source_ips: Vec<TopSource>,
    top_patterns: Vec<TopPattern>,
}

#[derive(Debug, Clone, Serialize)]
struct TopSource {
    source_ip: String,
    attempts: usize,
}

#[derive(Debug, Clone, Serialize)]
struct TopPattern {
    pattern: String,
    occurrences: usize,
}

#[derive(Debug, Default)]
struct SecurityStats {
    auth_probe_by_ip: HashMap<IpAddr, usize>,
    pattern_counts: HashMap<String, usize>,
}

#[derive(Clone)]
pub struct SmtpSecurityAgent {
    config: Arc<SmtpSecurityConfig>,
    state: Arc<Mutex<HashMap<IpAddr, SourceState>>>,
    stats: Arc<Mutex<SecurityStats>>,
    whitelist_nets: Arc<Vec<IpNet>>,
    whitelist_hosts: Arc<Vec<String>>,
}

impl SmtpSecurityAgent {
    pub async fn new() -> Self {
        let config = SmtpSecurityConfig::load().await;
        Self::from_config(config)
    }

    pub fn from_config(config: SmtpSecurityConfig) -> Self {
        let mut whitelist_nets = Vec::new();
        let mut whitelist_hosts = Vec::new();
        for entry in &config.whitelist {
            if let Ok(net) = entry.parse::<IpNet>() {
                whitelist_nets.push(net);
            } else if let Ok(ip) = entry.parse::<IpAddr>() {
                whitelist_nets.push(IpNet::from(ip));
            } else {
                whitelist_hosts.push(entry.to_ascii_lowercase());
            }
        }

        Self {
            config: Arc::new(config),
            state: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(SecurityStats::default())),
            whitelist_nets: Arc::new(whitelist_nets),
            whitelist_hosts: Arc::new(whitelist_hosts),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn observe_connection(&self, peer_addr: SocketAddr) -> SecurityDecision {
        if !self.config.enabled || self.is_whitelisted_ip(peer_addr.ip()) {
            return SecurityDecision::Allow;
        }

        let now = Utc::now();
        let mut state = self.state.lock().await;
        let source = state.entry(peer_addr.ip()).or_default();
        prune_before(&mut source.connections, now - ChronoDuration::hours(1));
        source.connections.push_back(now);

        if source.blocked_until.is_some_and(|until| until > now) {
            return SecurityDecision::Reject421(
                "421 Too many suspicious attempts, try again later\r\n".to_string(),
            );
        }

        let last_minute = source
            .connections
            .iter()
            .filter(|ts| **ts >= now - ChronoDuration::minutes(1))
            .count();
        if last_minute > self.config.max_connections_per_ip_per_minute
            || source.connections.len() > self.config.max_connections_per_ip_per_hour
        {
            source.risk_score += 30;
            drop(state);
            self.log_event(SmtpSecurityEvent {
                timestamp: now,
                event_type: "smtp_security_event".to_string(),
                source_ip: peer_addr.ip().to_string(),
                source_port: peer_addr.port(),
                reverse_dns: None,
                risk_score: 60,
                risk_level: "hostile".to_string(),
                reason: "SMTP connection rate limit exceeded".to_string(),
                action: "rate_limited".to_string(),
                block_duration: None,
                command: None,
                session_id: None,
            })
            .await;
            return SecurityDecision::Reject421(
                "421 Too many suspicious attempts, try again later\r\n".to_string(),
            );
        }

        SecurityDecision::Allow
    }

    pub async fn observe_command(
        &self,
        peer_addr: SocketAddr,
        session_id: &str,
        command_line: &str,
        command_count: usize,
        mail_delivery_attempted: bool,
    ) -> SecurityDecision {
        if !self.config.enabled {
            return SecurityDecision::Allow;
        }

        let command = command_line.trim();
        let command_upper = command.to_ascii_uppercase();
        let is_auth = command_upper.starts_with("AUTH ");
        let is_auth_login = command_upper.starts_with("AUTH LOGIN");
        let suspicious = is_auth
            || matches!(
                command_upper.split_whitespace().next(),
                Some("VRFY" | "EXPN" | "STARTTLS")
            );

        if !suspicious {
            return SecurityDecision::Allow;
        }

        let whitelisted = self.is_whitelisted_ip(peer_addr.ip());
        let now = Utc::now();
        let mut state = self.state.lock().await;
        let source = state.entry(peer_addr.ip()).or_default();
        prune_before(&mut source.auth_attempts, now - ChronoDuration::days(1));
        prune_before(
            &mut source.suspicious_commands,
            now - ChronoDuration::hours(1),
        );

        source.suspicious_commands.push_back(now);
        if is_auth {
            source.auth_attempts.push_back(now);
            source.risk_score += if is_auth_login { 30 } else { 20 };
        } else {
            source.risk_score += 10;
        }
        if command_count <= 2 && !mail_delivery_attempted {
            source.risk_score += 15;
        }
        if whitelisted {
            source.risk_score -= 100;
        }

        let auth_last_hour = source
            .auth_attempts
            .iter()
            .filter(|ts| **ts >= now - ChronoDuration::hours(1))
            .count();
        let risk_score = source.risk_score;
        let risk_level = risk_level(risk_score);

        let should_block = !whitelisted
            && self.config.temporary_block_enabled
            && is_auth_login
            && auth_last_hour > self.config.auth_login_threshold_per_hour;

        if should_block {
            source.blocked_until = Some(now + ChronoDuration::hours(1));
        }
        drop(state);

        let action = if whitelisted {
            "logged_whitelisted"
        } else if should_block {
            "temporary_block"
        } else if risk_score >= 60 {
            "rate_limited"
        } else {
            "logged"
        };

        let reason = if is_auth_login {
            "AUTH LOGIN attempted while AUTH is disabled"
        } else if is_auth {
            "Unsupported AUTH mechanism attempted while AUTH is disabled"
        } else {
            "Unsupported SMTP command used suspiciously"
        };

        self.record_pattern(
            peer_addr.ip(),
            if is_auth_login {
                "AUTH LOGIN probing"
            } else {
                "Suspicious SMTP command"
            },
        )
        .await;
        self.log_event(SmtpSecurityEvent {
            timestamp: now,
            event_type: "smtp_security_event".to_string(),
            source_ip: peer_addr.ip().to_string(),
            source_port: peer_addr.port(),
            reverse_dns: None,
            risk_score,
            risk_level: risk_level.to_string(),
            reason: reason.to_string(),
            action: action.to_string(),
            block_duration: should_block.then(|| self.config.malicious_block_duration.clone()),
            command: Some(command.to_string()),
            session_id: Some(session_id.to_string()),
        })
        .await;
        self.write_summary().await;

        if should_block {
            self.apply_temporary_block(peer_addr.ip()).await;
            return SecurityDecision::Reject421(
                "421 Too many suspicious attempts, try again later\r\n".to_string(),
            );
        }

        SecurityDecision::Allow
    }

    fn is_whitelisted_ip(&self, ip: IpAddr) -> bool {
        self.whitelist_nets.iter().any(|net| net.contains(&ip))
    }

    #[allow(dead_code)]
    fn is_whitelisted_hostname(&self, hostname: &str) -> bool {
        self.whitelist_hosts
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case(hostname))
    }

    async fn record_pattern(&self, ip: IpAddr, pattern: &str) {
        let mut stats = self.stats.lock().await;
        *stats.auth_probe_by_ip.entry(ip).or_default() += 1;
        *stats.pattern_counts.entry(pattern.to_string()).or_default() += 1;
    }

    async fn log_event(&self, event: SmtpSecurityEvent) {
        if let Some(parent) = self.config.event_log_path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }
        let Ok(line) = serde_json::to_string(&event) else {
            return;
        };
        append_jsonl(&self.config.event_log_path, &line).await;
    }

    async fn write_summary(&self) {
        if let Some(parent) = self.config.report_path.parent() {
            let _ = fs::create_dir_all(parent).await;
        }

        let stats = self.stats.lock().await;
        let mut top_source_ips: Vec<TopSource> = stats
            .auth_probe_by_ip
            .iter()
            .map(|(ip, attempts)| TopSource {
                source_ip: ip.to_string(),
                attempts: *attempts,
            })
            .collect();
        top_source_ips.sort_by(|a, b| b.attempts.cmp(&a.attempts));
        top_source_ips.truncate(10);

        let mut top_patterns: Vec<TopPattern> = stats
            .pattern_counts
            .iter()
            .map(|(pattern, occurrences)| TopPattern {
                pattern: pattern.clone(),
                occurrences: *occurrences,
            })
            .collect();
        top_patterns.sort_by(|a, b| b.occurrences.cmp(&a.occurrences));

        let suspicious_sessions = stats.auth_probe_by_ip.values().sum();
        let summary = SecuritySummary {
            timestamp: Utc::now(),
            event_type: "smtp_security_summary".to_string(),
            window: "process_lifetime".to_string(),
            suspicious_sessions,
            auth_probe_occurrences: suspicious_sessions,
            unique_source_ips: stats.auth_probe_by_ip.len(),
            top_source_ips,
            top_patterns,
        };
        drop(stats);

        if let Ok(line) = serde_json::to_string(&summary) {
            append_jsonl(&self.config.report_path, &line).await;
        }
    }

    async fn apply_temporary_block(&self, ip: IpAddr) {
        match self.config.blocking_backend {
            FirewallBackend::Disabled => {}
            FirewallBackend::Fail2banLog => {
                warn!("SMTP temporary block candidate for fail2ban: {}", ip);
            }
            FirewallBackend::HostAgent => {
                let request = FirewallRequest {
                    action: "block_ip".to_string(),
                    ip: Some(ip.to_string()),
                    duration: Some(self.config.malicious_block_duration.clone()),
                    reason: Some("AUTH LOGIN probing while AUTH is disabled".to_string()),
                    source: Some("smtp-detector".to_string()),
                };
                match send_request(&self.config.firewall_agent_socket_path, &request).await {
                    Ok(response) if response.status == "ok" => {}
                    Ok(response) => warn!(
                        "Host firewall agent rejected SMTP block for {}: {:?}",
                        ip, response.reason
                    ),
                    Err(e) => warn!("Host firewall agent request failed for {}: {}", ip, e),
                }
            }
            FirewallBackend::Nftables => {
                let status = Command::new("nft")
                    .args([
                        "add",
                        "element",
                        "inet",
                        "filter",
                        "smtp_blacklist",
                        &format!(
                            "{{ {} timeout {} }}",
                            ip, self.config.malicious_block_duration
                        ),
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                if let Err(e) = status {
                    warn!("Failed to apply nftables SMTP block for {}: {}", ip, e);
                }
            }
            FirewallBackend::Iptables => {
                let status = Command::new("iptables")
                    .args([
                        "-I",
                        "INPUT",
                        "-p",
                        "tcp",
                        "--dport",
                        "25",
                        "-s",
                        &ip.to_string(),
                        "-j",
                        "DROP",
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .await;
                if let Err(e) = status {
                    warn!("Failed to apply iptables SMTP block for {}: {}", ip, e);
                }
            }
        }
    }
}

fn prune_before(items: &mut VecDeque<DateTime<Utc>>, cutoff: DateTime<Utc>) {
    while items.front().is_some_and(|ts| *ts < cutoff) {
        items.pop_front();
    }
}

fn risk_level(score: i32) -> &'static str {
    match score {
        0..=29 => "normal",
        30..=59 => "suspicious",
        60..=99 => "hostile",
        _ => "malicious",
    }
}

async fn append_jsonl(path: &Path, line: &str) {
    match OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
    {
        Ok(mut file) => {
            let _ = file.write_all(line.as_bytes()).await;
            let _ = file.write_all(b"\n").await;
        }
        Err(e) => warn!("Failed to open SMTP security log {}: {}", path.display(), e),
    }
}

fn parse_bool(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn unquote(raw: &str) -> String {
    raw.trim().trim_matches('"').trim_matches('\'').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_supports_cidr_and_ips() {
        let agent = SmtpSecurityAgent::from_config(SmtpSecurityConfig {
            whitelist: vec!["192.168.0.0/16".to_string(), "127.0.0.1".to_string()],
            ..SmtpSecurityConfig::default()
        });

        assert!(agent.is_whitelisted_ip("192.168.1.10".parse().unwrap()));
        assert!(agent.is_whitelisted_ip("127.0.0.1".parse().unwrap()));
        assert!(!agent.is_whitelisted_ip("203.0.113.7".parse().unwrap()));
    }

    #[tokio::test]
    async fn auth_login_threshold_blocks_non_whitelisted_ip() {
        let agent = SmtpSecurityAgent::from_config(SmtpSecurityConfig {
            temporary_block_enabled: true,
            auth_login_threshold_per_hour: 1,
            whitelist: vec![],
            ..SmtpSecurityConfig::default()
        });
        let peer: SocketAddr = "203.0.113.7:53000".parse().unwrap();

        assert!(matches!(
            agent
                .observe_command(peer, "s1", "AUTH LOGIN", 1, false)
                .await,
            SecurityDecision::Allow
        ));
        assert!(matches!(
            agent
                .observe_command(peer, "s1", "AUTH LOGIN", 2, false)
                .await,
            SecurityDecision::Reject421(_)
        ));
    }

    #[tokio::test]
    async fn whitelisted_ip_is_logged_but_not_blocked() {
        let agent = SmtpSecurityAgent::from_config(SmtpSecurityConfig {
            temporary_block_enabled: true,
            auth_login_threshold_per_hour: 1,
            whitelist: vec!["203.0.113.0/24".to_string()],
            ..SmtpSecurityConfig::default()
        });
        let peer: SocketAddr = "203.0.113.7:53000".parse().unwrap();

        for i in 0..3 {
            assert!(matches!(
                agent
                    .observe_command(peer, "s1", "AUTH LOGIN", i + 1, false)
                    .await,
                SecurityDecision::Allow
            ));
        }
    }
}
