pub struct Config {
    pub database_url: String,
    pub server_port: u16,
    pub ai_api_key: String,
    pub developer_email: Option<String>,
    pub smtp_relay_host: Option<String>,
    pub smtp_relay_port: u16,
    pub smtp_relay_user: Option<String>,
    pub smtp_relay_pass: Option<String>,
    pub assistant_email: String,
    pub docs_whitelist: Vec<String>,
    pub readonly_mode_enabled: bool,
    pub readonly_base: Option<String>,
    pub overlay_dir: Option<String>,
    pub remote_debug_sshfs_enabled: bool,
    pub remote_debug_mode: String,
    pub remote_debug_remote: Option<String>,
    pub remote_debug_mount_point: Option<String>,
    pub remote_debug_overlay_dir: Option<String>,
}

impl Config {
    fn parse_bool_env(name: &str) -> bool {
        match std::env::var(name) {
            Ok(value) => {
                let normalized = value.trim().to_ascii_lowercase();
                matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
            }
            Err(_) => false,
        }
    }

    pub fn load() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data/data.sqlite".to_string()),
            server_port: std::env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
            ai_api_key: std::env::var("AI_API_KEY").unwrap_or_default(),
            developer_email: std::env::var("DEVELOPER_EMAIL").ok(),
            smtp_relay_host: std::env::var("SMTP_RELAY_HOST").ok(),
            smtp_relay_port: std::env::var("SMTP_RELAY_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse()
                .unwrap_or(587),
            smtp_relay_user: std::env::var("SMTP_RELAY_USER").ok(),
            smtp_relay_pass: std::env::var("SMTP_RELAY_PASS").ok(),
            assistant_email: std::env::var("ASSISTANT_EMAIL")
                .unwrap_or_else(|_| "assistant@example.com".to_string()),
            docs_whitelist: std::env::var("DOCS_WHITELIST")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            readonly_mode_enabled: Self::parse_bool_env("READONLY_MODE"),
            readonly_base: std::env::var("READONLY_BASE")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            overlay_dir: std::env::var("OVERLAY_DIR")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            remote_debug_sshfs_enabled: Self::parse_bool_env("REMOTE_DEBUG_SSHFS_ENABLED"),
            remote_debug_mode: std::env::var("REMOTE_DEBUG_MODE")
                .unwrap_or_else(|_| "readonly".to_string()),
            remote_debug_remote: std::env::var("REMOTE_DEBUG_REMOTE")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            remote_debug_mount_point: std::env::var("REMOTE_DEBUG_MOUNT_POINT")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            remote_debug_overlay_dir: std::env::var("REMOTE_DEBUG_OVERLAY_DIR")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_loads_with_defaults() {
        let config = Config::load();
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.smtp_relay_port, 587);
        assert_eq!(config.assistant_email, "assistant@example.com");
    }

    #[test]
    fn config_docs_whitelist_parses_comma_separated() {
        let input = "doc1.pdf,doc2.pdf,doc3.txt";
        let whitelist: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(whitelist.len(), 3);
    }

    #[test]
    fn config_docs_whitelist_handles_empty() {
        let input = "";
        let whitelist: Vec<String> = input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        assert!(whitelist.is_empty());
    }
}
