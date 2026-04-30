use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostFirewallBackend {
    Nftables,
    IptablesDockerUser,
    IptablesInput,
    Disabled,
}

impl HostFirewallBackend {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "nft" | "nftables" => Self::Nftables,
            "iptables" | "iptables-docker-user" | "docker-user" => Self::IptablesDockerUser,
            "iptables-input" | "input" => Self::IptablesInput,
            _ => Self::Disabled,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Nftables => "nftables",
            Self::IptablesDockerUser => "iptables_docker_user",
            Self::IptablesInput => "iptables_input",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FirewallAgentConfig {
    pub enabled: bool,
    pub socket_path: PathBuf,
    pub preferred_backend: HostFirewallBackend,
    pub fallback_backend: HostFirewallBackend,
    pub smtp_ports: Vec<u16>,
    pub default_duration: String,
    pub max_duration: String,
    pub allow_private_ip_blocking: bool,
    pub allow_cidr_blocking: bool,
    pub whitelist: Vec<String>,
    pub audit_log_path: PathBuf,
    pub state_path: PathBuf,
    pub cleanup_interval_seconds: u64,
}

impl Default for FirewallAgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            socket_path: PathBuf::from("/run/ai-mail-butler/firewall-agent.sock"),
            preferred_backend: HostFirewallBackend::Nftables,
            fallback_backend: HostFirewallBackend::IptablesDockerUser,
            smtp_ports: vec![25],
            default_duration: "1h".to_string(),
            max_duration: "24h".to_string(),
            allow_private_ip_blocking: false,
            allow_cidr_blocking: false,
            whitelist: vec![
                "127.0.0.1".to_string(),
                "::1".to_string(),
                "10.0.0.0/8".to_string(),
                "172.16.0.0/12".to_string(),
                "192.168.0.0/16".to_string(),
            ],
            audit_log_path: PathBuf::from("/var/log/ai-mail-butler/firewall-agent.jsonl"),
            state_path: PathBuf::from("/var/lib/ai-mail-butler/firewall-state.json"),
            cleanup_interval_seconds: 60,
        }
    }
}

impl FirewallAgentConfig {
    pub async fn load(path: Option<&str>) -> Self {
        let mut config = Self::default();
        let config_path = path
            .map(str::to_string)
            .or_else(|| std::env::var("FIREWALL_AGENT_CONFIG").ok())
            .unwrap_or_else(|| "/etc/ai-mail-butler/firewall-agent.yaml".to_string());

        if let Ok(raw) = fs::read_to_string(&config_path).await {
            config.apply_yaml_subset(&raw);
            info!("Loaded firewall agent config from {}", config_path);
        }

        if let Ok(v) = std::env::var("FIREWALL_AGENT_SOCKET") {
            config.socket_path = PathBuf::from(v);
        }
        if let Ok(v) = std::env::var("FIREWALL_AGENT_PREFERRED_BACKEND") {
            config.preferred_backend = HostFirewallBackend::parse(&v);
        }
        if let Ok(v) = std::env::var("FIREWALL_AGENT_FALLBACK_BACKEND") {
            config.fallback_backend = HostFirewallBackend::parse(&v);
        }

        config
    }

    fn apply_yaml_subset(&mut self, raw: &str) {
        let mut path: Vec<String> = Vec::new();
        let mut list_key: Option<String> = None;

        for original in raw.lines() {
            let no_comment = original.split('#').next().unwrap_or("").trim_end();
            if no_comment.trim().is_empty() {
                continue;
            }
            let indent = original.chars().take_while(|c| *c == ' ').count();
            let level = indent / 2;
            while path.len() > level {
                path.pop();
            }

            let trimmed = no_comment.trim();
            if trimmed.starts_with('-') {
                let value = unquote(trimmed.trim_start_matches('-').trim());
                match list_key.as_deref() {
                    Some("firewall_agent.smtp.ports") => {
                        if let Ok(port) = value.parse::<u16>() {
                            self.smtp_ports.push(port);
                        }
                    }
                    Some("firewall_agent.whitelist") => self.whitelist.push(value),
                    _ => {}
                }
                continue;
            }

            let Some((key, value)) = trimmed.split_once(':') else {
                continue;
            };
            let key = key.trim().to_string();
            let value = unquote(value.trim());

            if value.is_empty() {
                if path.len() == level {
                    path.push(key);
                } else if path.len() > level {
                    path[level] = key;
                }
                list_key = Some(path.join("."));
                continue;
            }

            let full_key = if path.is_empty() {
                key.clone()
            } else {
                format!("{}.{}", path.join("."), key)
            };
            match full_key.as_str() {
                "firewall_agent.enabled" => self.enabled = parse_bool(&value),
                "firewall_agent.socket_path" => self.socket_path = PathBuf::from(value),
                "firewall_agent.backend.preferred" => {
                    self.preferred_backend = HostFirewallBackend::parse(&value);
                }
                "firewall_agent.backend.fallback" => {
                    self.fallback_backend = HostFirewallBackend::parse(&value);
                }
                "firewall_agent.blocking.default_duration" => self.default_duration = value,
                "firewall_agent.blocking.max_duration" => self.max_duration = value,
                "firewall_agent.blocking.allow_private_ip_blocking" => {
                    self.allow_private_ip_blocking = parse_bool(&value);
                }
                "firewall_agent.blocking.allow_cidr_blocking" => {
                    self.allow_cidr_blocking = parse_bool(&value);
                }
                "firewall_agent.audit.log_path" => self.audit_log_path = PathBuf::from(value),
                "firewall_agent.state.path" => self.state_path = PathBuf::from(value),
                "firewall_agent.cleanup.interval_seconds" => {
                    if let Ok(v) = value.parse() {
                        self.cleanup_interval_seconds = v;
                    }
                }
                _ => {}
            }
        }

        self.smtp_ports.sort_unstable();
        self.smtp_ports.dedup();
        self.whitelist.sort();
        self.whitelist.dedup();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRequest {
    pub action: String,
    pub ip: Option<String>,
    pub duration: Option<String>,
    pub reason: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallResponse {
    pub status: String,
    pub ip: Option<String>,
    pub reason: Option<String>,
    pub backend: Option<String>,
    pub firewall_ready: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
    pub blocked: Option<Vec<BlockRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockRecord {
    pub ip: IpAddr,
    pub reason: String,
    pub source: String,
    pub backend: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct FirewallState {
    blocked: HashMap<IpAddr, BlockRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct AuditEvent {
    timestamp: DateTime<Utc>,
    event_type: String,
    ip: Option<String>,
    duration: Option<String>,
    reason: String,
    source: Option<String>,
    backend: Option<String>,
    result: String,
    expires_at: Option<DateTime<Utc>>,
}

pub struct FirewallAgent {
    config: Arc<FirewallAgentConfig>,
    whitelist: Vec<IpNet>,
    state: Arc<Mutex<FirewallState>>,
}

impl FirewallAgent {
    pub async fn new(config: FirewallAgentConfig) -> Self {
        let whitelist = config
            .whitelist
            .iter()
            .filter_map(|entry| {
                entry
                    .parse::<IpNet>()
                    .ok()
                    .or_else(|| entry.parse::<IpAddr>().ok().map(IpNet::from))
            })
            .collect();
        let state = load_state(&config.state_path).await.unwrap_or_default();
        Self {
            config: Arc::new(config),
            whitelist,
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub async fn run(self) -> Result<()> {
        if !self.config.enabled {
            return Err(anyhow!("firewall agent is disabled"));
        }
        if let Some(parent) = self.config.socket_path.parent() {
            fs::create_dir_all(parent).await?;
        }
        if fs::try_exists(&self.config.socket_path).await.unwrap_or(false) {
            fs::remove_file(&self.config.socket_path).await?;
        }

        self.ensure_firewall_ready().await?;
        self.restore_active_blocks().await;

        let listener = UnixListener::bind(&self.config.socket_path)?;
        info!(
            "Firewall agent listening on {}",
            self.config.socket_path.display()
        );

        let cleanup_agent = self.clone_for_task();
        tokio::spawn(async move {
            cleanup_agent.cleanup_loop().await;
        });

        loop {
            let (stream, _) = listener.accept().await?;
            let agent = self.clone_for_task();
            tokio::spawn(async move {
                if let Err(e) = agent.handle_stream(stream).await {
                    warn!("firewall agent request failed: {}", e);
                }
            });
        }
    }

    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            whitelist: self.whitelist.clone(),
            state: self.state.clone(),
        }
    }

    async fn handle_stream(&self, stream: UnixStream) -> Result<()> {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        let response = match serde_json::from_str::<FirewallRequest>(&line) {
            Ok(request) => self.handle_request(request).await,
            Err(e) => FirewallResponse {
                status: "error".to_string(),
                ip: None,
                reason: Some(format!("invalid json request: {}", e)),
                backend: None,
                firewall_ready: None,
                expires_at: None,
                blocked: None,
            },
        };
        let mut stream = reader.into_inner();
        stream
            .write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes())
            .await?;
        Ok(())
    }

    async fn handle_request(&self, request: FirewallRequest) -> FirewallResponse {
        match request.action.as_str() {
            "health" => FirewallResponse {
                status: "ok".to_string(),
                ip: None,
                reason: None,
                backend: Some(self.config.preferred_backend.as_str().to_string()),
                firewall_ready: Some(self.ensure_firewall_ready().await.is_ok()),
                expires_at: None,
                blocked: None,
            },
            "list_blocks" => {
                let blocked = self
                    .state
                    .lock()
                    .await
                    .blocked
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                FirewallResponse {
                    status: "ok".to_string(),
                    ip: None,
                    reason: None,
                    backend: Some(self.config.preferred_backend.as_str().to_string()),
                    firewall_ready: None,
                    expires_at: None,
                    blocked: Some(blocked),
                }
            }
            "block_ip" => self.block_ip(request).await,
            "unblock_ip" => self.unblock_ip(request).await,
            _ => FirewallResponse {
                status: "error".to_string(),
                ip: request.ip,
                reason: Some("unsupported action".to_string()),
                backend: None,
                firewall_ready: None,
                expires_at: None,
                blocked: None,
            },
        }
    }

    async fn block_ip(&self, request: FirewallRequest) -> FirewallResponse {
        let ip_raw = request.ip.clone().unwrap_or_default();
        let reason = request.reason.clone().unwrap_or_default();
        let source = request.source.clone().unwrap_or_else(|| "unknown".to_string());
        let duration_raw = request
            .duration
            .clone()
            .unwrap_or_else(|| self.config.default_duration.clone());

        let ip = match validate_ip(&ip_raw) {
            Ok(ip) => ip,
            Err(e) => return self.rejected(request, e.to_string()).await,
        };
        if reason.trim().is_empty() || reason.chars().count() > 240 {
            return self
                .rejected(request, "reason must be 1-240 characters".to_string())
                .await;
        }
        if self.is_whitelisted(ip) {
            return self.rejected(request, "ip is whitelisted".to_string()).await;
        }
        if !self.config.allow_private_ip_blocking && is_private_or_local(ip) {
            return self
                .rejected(request, "private/internal IP blocking is disabled".to_string())
                .await;
        }

        let duration = match parse_duration(&duration_raw) {
            Some(v) => v,
            None => return self.rejected(request, "invalid duration".to_string()).await,
        };
        let max_duration = parse_duration(&self.config.max_duration)
            .unwrap_or_else(|| ChronoDuration::hours(24));
        if duration > max_duration {
            return self.rejected(request, "duration exceeds max_duration".to_string()).await;
        }

        let expires_at = Utc::now() + duration;
        match self.apply_block(ip, &duration_raw).await {
            Ok(backend) => {
                let record = BlockRecord {
                    ip,
                    reason: reason.clone(),
                    source: source.clone(),
                    backend: backend.as_str().to_string(),
                    expires_at,
                };
                {
                    let mut state = self.state.lock().await;
                    state.blocked.insert(ip, record);
                    let _ = save_state(&self.config.state_path, &state).await;
                }
                self.audit("firewall_block", Some(ip), Some(duration_raw), reason, Some(source), Some(backend), "success", Some(expires_at)).await;
                FirewallResponse {
                    status: "ok".to_string(),
                    ip: Some(ip.to_string()),
                    reason: None,
                    backend: Some(backend.as_str().to_string()),
                    firewall_ready: None,
                    expires_at: Some(expires_at),
                    blocked: None,
                }
            }
            Err(e) => {
                self.audit("firewall_block_failed", Some(ip), Some(duration_raw), e.to_string(), Some(source), None, "error", None).await;
                FirewallResponse {
                    status: "error".to_string(),
                    ip: Some(ip.to_string()),
                    reason: Some(e.to_string()),
                    backend: None,
                    firewall_ready: None,
                    expires_at: None,
                    blocked: None,
                }
            }
        }
    }

    async fn unblock_ip(&self, request: FirewallRequest) -> FirewallResponse {
        let ip_raw = request.ip.clone().unwrap_or_default();
        let ip = match validate_ip(&ip_raw) {
            Ok(ip) => ip,
            Err(e) => return self.rejected(request, e.to_string()).await,
        };
        let reason = request
            .reason
            .clone()
            .unwrap_or_else(|| "manual unblock".to_string());

        let backend = {
            let mut state = self.state.lock().await;
            let backend = state
                .blocked
                .remove(&ip)
                .map(|record| HostFirewallBackend::parse(&record.backend))
                .unwrap_or(self.config.preferred_backend);
            let _ = save_state(&self.config.state_path, &state).await;
            backend
        };
        let _ = self.apply_unblock(ip, backend).await;
        self.audit("firewall_unblock", Some(ip), None, reason, request.source, Some(backend), "success", None).await;
        FirewallResponse {
            status: "ok".to_string(),
            ip: Some(ip.to_string()),
            reason: None,
            backend: Some(backend.as_str().to_string()),
            firewall_ready: None,
            expires_at: None,
            blocked: None,
        }
    }

    async fn rejected(&self, request: FirewallRequest, reason: String) -> FirewallResponse {
        let ip = request.ip.as_deref().and_then(|raw| raw.parse().ok());
        self.audit("firewall_block_rejected", ip, request.duration, reason.clone(), request.source, None, "rejected", None).await;
        FirewallResponse {
            status: "rejected".to_string(),
            ip: request.ip,
            reason: Some(reason),
            backend: None,
            firewall_ready: None,
            expires_at: None,
            blocked: None,
        }
    }

    async fn ensure_firewall_ready(&self) -> Result<()> {
        match self.config.preferred_backend {
            HostFirewallBackend::Nftables => self.ensure_nftables().await,
            HostFirewallBackend::IptablesDockerUser => self.ensure_iptables_docker_user().await,
            HostFirewallBackend::IptablesInput => Ok(()),
            HostFirewallBackend::Disabled => Ok(()),
        }
    }

    async fn ensure_nftables(&self) -> Result<()> {
        let _ = run_command("nft", &["add", "table", "inet", "ai_mail_butler"]).await;
        let _ = run_command(
            "nft",
            &[
                "add",
                "set",
                "inet",
                "ai_mail_butler",
                "blocked_ipv4",
                "{ type ipv4_addr; flags timeout; }",
            ],
        )
        .await;
        let _ = run_command(
            "nft",
            &[
                "add",
                "set",
                "inet",
                "ai_mail_butler",
                "blocked_ipv6",
                "{ type ipv6_addr; flags timeout; }",
            ],
        )
        .await;
        let _ = run_command(
            "nft",
            &[
                "add",
                "chain",
                "inet",
                "ai_mail_butler",
                "input",
                "{ type filter hook input priority -100; policy accept; }",
            ],
        )
        .await;
        for port in &self.config.smtp_ports {
            let _ = run_command(
                "nft",
                &[
                    "add",
                    "rule",
                    "inet",
                    "ai_mail_butler",
                    "input",
                    "tcp",
                    "dport",
                    &port.to_string(),
                    "ip",
                    "saddr",
                    "@blocked_ipv4",
                    "drop",
                ],
            )
            .await;
            let _ = run_command(
                "nft",
                &[
                    "add",
                    "rule",
                    "inet",
                    "ai_mail_butler",
                    "input",
                    "tcp",
                    "dport",
                    &port.to_string(),
                    "ip6",
                    "saddr",
                    "@blocked_ipv6",
                    "drop",
                ],
            )
            .await;
        }
        Ok(())
    }

    async fn ensure_iptables_docker_user(&self) -> Result<()> {
        run_command("iptables", &["-N", "DOCKER-USER"]).await.ok();
        Ok(())
    }

    async fn apply_block(&self, ip: IpAddr, duration: &str) -> Result<HostFirewallBackend> {
        let backends = [
            self.config.preferred_backend,
            self.config.fallback_backend,
            HostFirewallBackend::IptablesInput,
        ];
        let mut last_error = None;
        for backend in backends {
            if backend == HostFirewallBackend::Disabled {
                continue;
            }
            let result = match backend {
                HostFirewallBackend::Nftables => {
                    let set_name = if ip.is_ipv4() {
                        "blocked_ipv4"
                    } else {
                        "blocked_ipv6"
                    };
                    run_command(
                        "nft",
                        &[
                            "add",
                            "element",
                            "inet",
                            "ai_mail_butler",
                            set_name,
                            &format!("{{ {} timeout {} }}", ip, duration),
                        ],
                    )
                    .await
                }
                HostFirewallBackend::IptablesDockerUser => {
                    self.apply_iptables_rule("DOCKER-USER", ip).await
                }
                HostFirewallBackend::IptablesInput => self.apply_iptables_rule("INPUT", ip).await,
                HostFirewallBackend::Disabled => Ok(()),
            };
            match result {
                Ok(_) => return Ok(backend),
                Err(e) => last_error = Some(e),
            }
        }
        Err(last_error.unwrap_or_else(|| anyhow!("no available firewall backend")))
    }

    async fn apply_iptables_rule(&self, chain: &str, ip: IpAddr) -> Result<()> {
        let program = if ip.is_ipv4() { "iptables" } else { "ip6tables" };
        for port in &self.config.smtp_ports {
            run_command(
                program,
                &[
                    "-I",
                    chain,
                    "-s",
                    &ip.to_string(),
                    "-p",
                    "tcp",
                    "--dport",
                    &port.to_string(),
                    "-j",
                    "DROP",
                ],
            )
            .await?;
        }
        Ok(())
    }

    async fn apply_unblock(&self, ip: IpAddr, backend: HostFirewallBackend) -> Result<()> {
        match backend {
            HostFirewallBackend::Nftables => {
                let set_name = if ip.is_ipv4() {
                    "blocked_ipv4"
                } else {
                    "blocked_ipv6"
                };
                run_command(
                    "nft",
                    &[
                        "delete",
                        "element",
                        "inet",
                        "ai_mail_butler",
                        set_name,
                        &format!("{{ {} }}", ip),
                    ],
                )
                .await
            }
            HostFirewallBackend::IptablesDockerUser => self.delete_iptables_rule("DOCKER-USER", ip).await,
            HostFirewallBackend::IptablesInput => self.delete_iptables_rule("INPUT", ip).await,
            HostFirewallBackend::Disabled => Ok(()),
        }
    }

    async fn delete_iptables_rule(&self, chain: &str, ip: IpAddr) -> Result<()> {
        let program = if ip.is_ipv4() { "iptables" } else { "ip6tables" };
        for port in &self.config.smtp_ports {
            let _ = run_command(
                program,
                &[
                    "-D",
                    chain,
                    "-s",
                    &ip.to_string(),
                    "-p",
                    "tcp",
                    "--dport",
                    &port.to_string(),
                    "-j",
                    "DROP",
                ],
            )
            .await;
        }
        Ok(())
    }

    async fn restore_active_blocks(&self) {
        let now = Utc::now();
        let records = self
            .state
            .lock()
            .await
            .blocked
            .values()
            .filter(|record| record.expires_at > now)
            .cloned()
            .collect::<Vec<_>>();
        for record in records {
            let remaining = record.expires_at - now;
            let duration = format!("{}s", remaining.num_seconds().max(1));
            if let Err(e) = self.apply_block(record.ip, &duration).await {
                warn!("failed to restore firewall block for {}: {}", record.ip, e);
            }
        }
        self.cleanup_expired().await;
    }

    async fn cleanup_loop(&self) {
        loop {
            sleep(Duration::from_secs(self.config.cleanup_interval_seconds)).await;
            self.cleanup_expired().await;
        }
    }

    async fn cleanup_expired(&self) {
        let now = Utc::now();
        let expired = {
            let mut state = self.state.lock().await;
            let expired = state
                .blocked
                .values()
                .filter(|record| record.expires_at <= now)
                .cloned()
                .collect::<Vec<_>>();
            for record in &expired {
                state.blocked.remove(&record.ip);
            }
            let _ = save_state(&self.config.state_path, &state).await;
            expired
        };
        for record in expired {
            let backend = HostFirewallBackend::parse(&record.backend);
            let _ = self.apply_unblock(record.ip, backend).await;
            self.audit("firewall_block_expired", Some(record.ip), None, record.reason, Some(record.source), Some(backend), "success", None).await;
        }
    }

    fn is_whitelisted(&self, ip: IpAddr) -> bool {
        self.whitelist.iter().any(|net| net.contains(&ip))
    }

    async fn audit(
        &self,
        event_type: &str,
        ip: Option<IpAddr>,
        duration: Option<String>,
        reason: String,
        source: Option<String>,
        backend: Option<HostFirewallBackend>,
        result: &str,
        expires_at: Option<DateTime<Utc>>,
    ) {
        let event = AuditEvent {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            ip: ip.map(|v| v.to_string()),
            duration,
            reason,
            source,
            backend: backend.map(|v| v.as_str().to_string()),
            result: result.to_string(),
            expires_at,
        };
        if let Ok(line) = serde_json::to_string(&event) {
            append_jsonl(&self.config.audit_log_path, &line).await;
        }
    }
}

pub async fn send_request(socket_path: &Path, request: &FirewallRequest) -> Result<FirewallResponse> {
    let mut stream = UnixStream::connect(socket_path).await?;
    stream
        .write_all(format!("{}\n", serde_json::to_string(request)?).as_bytes())
        .await?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    Ok(serde_json::from_str(&line)?)
}

fn validate_ip(raw: &str) -> Result<IpAddr> {
    if raw.contains('/') {
        return Err(anyhow!("CIDR block requests are disabled"));
    }
    raw.parse::<IpAddr>()
        .map_err(|_| anyhow!("ip must be a valid IPv4 or IPv6 address"))
}

fn is_private_or_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local(),
    }
}

fn parse_duration(raw: &str) -> Option<ChronoDuration> {
    let trimmed = raw.trim();
    let (number, unit) = trimmed.split_at(trimmed.len().saturating_sub(1));
    let value = number.parse::<i64>().ok()?;
    match unit {
        "s" => Some(ChronoDuration::seconds(value)),
        "m" => Some(ChronoDuration::minutes(value)),
        "h" => Some(ChronoDuration::hours(value)),
        "d" => Some(ChronoDuration::days(value)),
        _ => None,
    }
    .filter(|duration| *duration > ChronoDuration::zero())
}

async fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .await?;
    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "{} failed: {}",
            program,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

async fn load_state(path: &Path) -> Result<FirewallState> {
    let raw = fs::read_to_string(path).await?;
    Ok(serde_json::from_str(&raw)?)
}

async fn save_state(path: &Path, state: &FirewallState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(path, serde_json::to_string_pretty(state)?).await?;
    Ok(())
}

async fn append_jsonl(path: &Path, line: &str) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent).await;
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path).await {
        let _ = file.write_all(line.as_bytes()).await;
        let _ = file.write_all(b"\n").await;
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
    fn duration_parser_supports_common_units() {
        assert_eq!(parse_duration("60s"), Some(ChronoDuration::seconds(60)));
        assert_eq!(parse_duration("10m"), Some(ChronoDuration::minutes(10)));
        assert_eq!(parse_duration("1h"), Some(ChronoDuration::hours(1)));
        assert_eq!(parse_duration("1d"), Some(ChronoDuration::days(1)));
        assert_eq!(parse_duration("0h"), None);
    }

    #[test]
    fn validation_rejects_cidr_and_hostnames() {
        assert!(validate_ip("158.94.208.79").is_ok());
        assert!(validate_ip("10.0.0.0/8").is_err());
        assert!(validate_ip("mail.example.com").is_err());
    }

    #[tokio::test]
    async fn whitelisted_block_is_rejected() {
        let agent = FirewallAgent::new(FirewallAgentConfig {
            preferred_backend: HostFirewallBackend::Disabled,
            whitelist: vec!["127.0.0.1".to_string()],
            ..FirewallAgentConfig::default()
        })
        .await;
        let response = agent
            .handle_request(FirewallRequest {
                action: "block_ip".to_string(),
                ip: Some("127.0.0.1".to_string()),
                duration: Some("1h".to_string()),
                reason: Some("test".to_string()),
                source: Some("test".to_string()),
            })
            .await;
        assert_eq!(response.status, "rejected");
    }
}
