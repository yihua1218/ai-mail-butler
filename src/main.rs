mod ai;
mod config;
mod db;
mod mail;
mod models;
mod services;
mod web;

use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize environment variables
    dotenvy::dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting AI Mail Butler...");

    let config = config::Config::load();

    // 1. Initialize Database
    let pool = db::connect(&config.database_url).await?;
    info!("Database connected successfully.");

    // 2. Initialize AI Client
    let ai_client = ai::AiClient::new(&config);

    // 3. Start SMTP Server
    tokio::spawn({
        let pool = pool.clone();
        let ai_client = ai_client.clone();
        async move {
            if let Err(e) = mail::MailService::start(pool, ai_client).await {
                tracing::error!("Mail server failed: {}", e);
            }
        }
    });

    // 4. Start Web Server
    let state = web::AppState { pool, ai_client };
    web::start_server(config.server_port, state).await?;

    Ok(())
}
