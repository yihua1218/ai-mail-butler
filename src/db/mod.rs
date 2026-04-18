use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use anyhow::Result;

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY NOT NULL,
            email TEXT UNIQUE NOT NULL,
            is_onboarded BOOLEAN NOT NULL DEFAULT 0,
            preferences TEXT
        );"
    )
    .execute(&pool)
    .await?;

    // Safely add columns if they don't exist (idempotent migrations)
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN magic_token TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN auto_reply BOOLEAN NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN dry_run BOOLEAN NOT NULL DEFAULT 1").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN display_name TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN role TEXT").execute(&pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS emails (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            subject TEXT,
            preview TEXT,
            status TEXT NOT NULL,
            received_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );"
    )
    .execute(&pool)
    .await?;

    // Track AI assistant chat replies (anonymous + logged-in)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_email TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn run_startup_diagnostics(pool: &SqlitePool) -> Result<()> {
    use crate::models::User;
    tracing::info!("Running startup diagnostics...");

    // Test 1: Verify User model mapping (checks for ColumnNotFound errors)
    let test_user = sqlx::query_as::<_, User>("SELECT * FROM users LIMIT 1")
        .fetch_optional(pool)
        .await;

    match test_user {
        Ok(_) => tracing::info!("Startup Diagnostic: User model mapping verified successfully."),
        Err(e) => {
            tracing::error!("Startup Diagnostic FAILED: Database schema mismatch with User model: {:?}", e);
            return Err(anyhow::anyhow!("Database schema mismatch: {}", e));
        }
    }

    Ok(())
}
