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

    // TODO: Initialize database connection
    // TODO: Start SMTP server / Webhook receiver
    // TODO: Start Web Server (Axum)

    Ok(())
}
