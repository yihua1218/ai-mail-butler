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

    // Safely add magic_token column if it doesn't exist
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN magic_token TEXT").execute(&pool).await;
    // Safely add auto_reply and dry_run columns
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN auto_reply BOOLEAN NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN dry_run BOOLEAN NOT NULL DEFAULT 1").execute(&pool).await;

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

    Ok(pool)
}
