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
}

impl Config {
    pub fn load() -> Self {
        // Load configurations from env variables
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/data.sqlite".to_string()),
            server_port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse().unwrap_or(3000),
            ai_api_key: std::env::var("AI_API_KEY").unwrap_or_default(),
            developer_email: std::env::var("DEVELOPER_EMAIL").ok(),
            smtp_relay_host: std::env::var("SMTP_RELAY_HOST").ok(),
            smtp_relay_port: std::env::var("SMTP_RELAY_PORT").unwrap_or_else(|_| "587".to_string()).parse().unwrap_or(587),
            smtp_relay_user: std::env::var("SMTP_RELAY_USER").ok(),
            smtp_relay_pass: std::env::var("SMTP_RELAY_PASS").ok(),
            assistant_email: std::env::var("ASSISTANT_EMAIL").unwrap_or_else(|_| "assistant@example.com".to_string()),
            docs_whitelist: std::env::var("DOCS_WHITELIST")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
        }
    }
}
