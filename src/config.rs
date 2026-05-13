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
    pub readonly_block_writes: bool,
    pub readonly_base: Option<String>,
    pub overlay_dir: Option<String>,
    pub remote_debug_sshfs_enabled: bool,
    pub remote_debug_mode: String,
    pub remote_debug_access_mode: String,
    pub remote_debug_remote: Option<String>,
    pub remote_debug_mount_point: Option<String>,
    pub remote_debug_overlay_dir: Option<String>,
    pub cloudflare_zone_id: Option<String>,
    pub cloudflare_api_token: Option<String>,
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

    fn parse_optional_bool_env(name: &str) -> Option<bool> {
        std::env::var(name).ok().map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
    }

    pub fn load() -> Self {
        let readonly_mode_enabled = Self::parse_bool_env("READONLY_MODE");
        let readonly_block_writes =
            Self::parse_optional_bool_env("READONLY_BLOCK_WRITES").unwrap_or(readonly_mode_enabled);

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
            readonly_mode_enabled,
            readonly_block_writes,
            readonly_base: std::env::var("READONLY_BASE")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            overlay_dir: std::env::var("OVERLAY_DIR")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            remote_debug_sshfs_enabled: Self::parse_bool_env("REMOTE_DEBUG_SSHFS_ENABLED"),
            remote_debug_mode: std::env::var("REMOTE_DEBUG_MODE")
                .unwrap_or_else(|_| "readonly".to_string()),
            remote_debug_access_mode: std::env::var("REMOTE_DEBUG_ACCESS_MODE")
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
            cloudflare_zone_id: std::env::var("CLOUDFLARE_ZONE_ID")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            cloudflare_api_token: std::env::var("CLOUDFLARE_API_TOKEN")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
    }

    fn clear_config_env() {
        for key in [
            "DATABASE_URL",
            "PORT",
            "AI_API_KEY",
            "DEVELOPER_EMAIL",
            "SMTP_RELAY_HOST",
            "SMTP_RELAY_PORT",
            "SMTP_RELAY_USER",
            "SMTP_RELAY_PASS",
            "ASSISTANT_EMAIL",
            "DOCS_WHITELIST",
            "READONLY_MODE",
            "READONLY_BLOCK_WRITES",
            "READONLY_BASE",
            "OVERLAY_DIR",
            "REMOTE_DEBUG_SSHFS_ENABLED",
            "REMOTE_DEBUG_MODE",
            "REMOTE_DEBUG_ACCESS_MODE",
            "REMOTE_DEBUG_REMOTE",
            "REMOTE_DEBUG_MOUNT_POINT",
            "REMOTE_DEBUG_OVERLAY_DIR",
            "CLOUDFLARE_ZONE_ID",
            "CLOUDFLARE_API_TOKEN",
        ] {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn config_loads_with_defaults() {
        let _guard = env_lock().lock().expect("lock config env");
        clear_config_env();
        let config = Config::load();
        assert_eq!(config.database_url, "sqlite:data/data.sqlite");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.smtp_relay_port, 587);
        assert_eq!(config.assistant_email, "assistant@example.com");
        assert!(!config.readonly_mode_enabled);
        assert!(!config.readonly_block_writes);
        assert_eq!(config.remote_debug_mode, "readonly");
        assert_eq!(config.remote_debug_access_mode, "readonly");
    }

    #[test]
    fn config_load_parses_runtime_env_values() {
        let _guard = env_lock().lock().expect("lock config env");
        clear_config_env();
        std::env::set_var("DATABASE_URL", "sqlite:/app/data/data.sqlite");
        std::env::set_var("PORT", "8080");
        std::env::set_var("SMTP_RELAY_PORT", "2525");
        std::env::set_var("ASSISTANT_EMAIL", "assistant@mail.example.com");
        std::env::set_var("DOCS_WHITELIST", " RBAC.md, zh-TW ,,SMTP ");
        std::env::set_var("READONLY_MODE", "yes");
        std::env::set_var("READONLY_BLOCK_WRITES", "false");
        std::env::set_var("READONLY_BASE", "/mnt/base");
        std::env::set_var("OVERLAY_DIR", "/tmp/overlay");
        std::env::set_var("REMOTE_DEBUG_SSHFS_ENABLED", "on");
        std::env::set_var("REMOTE_DEBUG_MODE", "overlay");
        std::env::set_var("REMOTE_DEBUG_ACCESS_MODE", "readwrite");
        std::env::set_var("REMOTE_DEBUG_REMOTE", "ec2:/srv/data");
        std::env::set_var("REMOTE_DEBUG_MOUNT_POINT", "/mnt/ai-mail-butler-data");
        std::env::set_var("REMOTE_DEBUG_OVERLAY_DIR", "/tmp/remote-overlay");
        std::env::set_var("CLOUDFLARE_ZONE_ID", "zone-id");
        std::env::set_var("CLOUDFLARE_API_TOKEN", "token");

        let config = Config::load();
        assert_eq!(config.database_url, "sqlite:/app/data/data.sqlite");
        assert_eq!(config.server_port, 8080);
        assert_eq!(config.smtp_relay_port, 2525);
        assert_eq!(config.assistant_email, "assistant@mail.example.com");
        assert_eq!(config.docs_whitelist, vec!["RBAC.md", "zh-TW", "SMTP"]);
        assert!(config.readonly_mode_enabled);
        assert!(!config.readonly_block_writes);
        assert_eq!(config.readonly_base.as_deref(), Some("/mnt/base"));
        assert_eq!(config.overlay_dir.as_deref(), Some("/tmp/overlay"));
        assert!(config.remote_debug_sshfs_enabled);
        assert_eq!(config.remote_debug_mode, "overlay");
        assert_eq!(config.remote_debug_access_mode, "readwrite");
        assert_eq!(config.remote_debug_remote.as_deref(), Some("ec2:/srv/data"));
        assert_eq!(
            config.remote_debug_mount_point.as_deref(),
            Some("/mnt/ai-mail-butler-data")
        );
        assert_eq!(
            config.remote_debug_overlay_dir.as_deref(),
            Some("/tmp/remote-overlay")
        );
        assert_eq!(config.cloudflare_zone_id.as_deref(), Some("zone-id"));
        assert_eq!(config.cloudflare_api_token.as_deref(), Some("token"));
    }

    #[test]
    fn config_readonly_block_writes_defaults_to_readonly_mode() {
        let _guard = env_lock().lock().expect("lock config env");
        clear_config_env();
        std::env::set_var("READONLY_MODE", "true");

        let config = Config::load();
        assert!(config.readonly_mode_enabled);
        assert!(config.readonly_block_writes);
    }
}
