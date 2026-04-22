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
    db::run_startup_diagnostics(&pool).await?;

    // Run initial data retention cleanup and schedule it every 24h
    {
        let pool_cleanup = pool.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = db::cleanup_expired_data(&pool_cleanup).await {
                    tracing::warn!("cleanup_expired_data error: {}", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;
            }
        });
    }

    // 2. Initialize AI Client
    let ai_client = ai::AiClient::new(&config);

    let config_arc = std::sync::Arc::new(config);

    // 3. Start SMTP Server
    tokio::spawn({
        let pool = pool.clone();
        let ai_client = ai_client.clone();
        let config_clone = config_arc.clone();
        async move {
            if let Err(e) = mail::MailService::start(pool, ai_client, config_clone).await {
                tracing::error!("Mail server failed: {}", e);
            }
        }
    });

    // 4. Start Web Server
    let admin_email = std::env::var("ADMIN_EMAIL").ok();
    let developer_email = config_arc.developer_email.clone();
    let state = web::AppState { pool, ai_client, admin_email, developer_email, config: config_arc.clone() };
    web::start_server(state.config.server_port, state).await?;

    Ok(())
}
