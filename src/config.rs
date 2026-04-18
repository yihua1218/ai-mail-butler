pub struct Config {
    pub database_url: String,
    pub server_port: u16,
    pub ai_api_key: String,
    // Add other config fields...
}

impl Config {
    pub fn load() -> Self {
        // Load configurations from env variables
        Self {
            database_url: std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:data/data.sqlite".to_string()),
            server_port: std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()).parse().unwrap_or(3000),
            ai_api_key: std::env::var("AI_API_KEY").unwrap_or_default(),
        }
    }
}
