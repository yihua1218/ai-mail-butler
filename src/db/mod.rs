use sqlx::{sqlite::{SqliteConnectOptions, SqlitePoolOptions}, SqlitePool};
use anyhow::Result;
use std::str::FromStr;

pub async fn connect(database_url: &str) -> Result<SqlitePool> {
    // Ensure the parent directory exists before SQLite tries to create the file.
    // Strips the "sqlite:" prefix to get the file path.
    let file_path = database_url
        .trim_start_matches("sqlite:")
        .trim_start_matches("//");
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
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
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN email_format TEXT NOT NULL DEFAULT 'both'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN assistant_name_zh TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN assistant_name_en TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN assistant_tone_zh TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN assistant_tone_en TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN onboarding_step INTEGER NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN pdf_passwords TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN timezone TEXT NOT NULL DEFAULT 'UTC'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN preferred_language TEXT NOT NULL DEFAULT 'en'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN training_data_consent BOOLEAN NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN training_consent_updated_at DATETIME").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN mail_send_method TEXT NOT NULL DEFAULT 'direct_mx'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE users ADD COLUMN rule_label_mode TEXT NOT NULL DEFAULT 'ai_first'").execute(&pool).await;

    // Table for tracking repetitive behaviors/questions for analytics
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_activity_stats (
            user_id TEXT NOT NULL,
            activity_key TEXT NOT NULL,
            count INTEGER NOT NULL DEFAULT 1,
            last_occurred DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY(user_id, activity_key)
        );"
    )
    .execute(&pool)
    .await?;

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

    let _ = sqlx::query("ALTER TABLE emails ADD COLUMN stored_content TEXT").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_emails_user_received ON emails(user_id, received_at DESC)").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_emails_user_status_received ON emails(user_id, status, received_at DESC)").execute(&pool).await;

    // User-defined and chat-captured email processing rules.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS email_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            rule_text TEXT NOT NULL,
            rule_label TEXT NOT NULL DEFAULT 'RULE',
            source TEXT NOT NULL DEFAULT 'manual',
            is_enabled BOOLEAN NOT NULL DEFAULT 1,
            matched_count INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );"
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE email_rules ADD COLUMN rule_label TEXT NOT NULL DEFAULT 'RULE'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_rules ADD COLUMN matched_count INTEGER NOT NULL DEFAULT 0").execute(&pool).await;

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

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_transcripts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT,
            user_email TEXT,
            user_message TEXT NOT NULL,
            ai_reply TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(&pool)
    .await?;

    // User feedback for AI replies, including optional improvement suggestions.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS chat_feedback (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_email TEXT,
            ai_reply TEXT NOT NULL,
            rating TEXT NOT NULL,
            suggestion TEXT,
            is_read BOOLEAN NOT NULL DEFAULT 0,
            read_at DATETIME,
            admin_reply TEXT,
            replied_at DATETIME,
            replied_by TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE chat_feedback ADD COLUMN is_read BOOLEAN NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE chat_feedback ADD COLUMN read_at DATETIME").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE chat_feedback ADD COLUMN admin_reply TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE chat_feedback ADD COLUMN replied_at DATETIME").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE chat_feedback ADD COLUMN replied_by TEXT").execute(&pool).await;

    let _ = sqlx::query("ALTER TABLE emails ADD COLUMN matched_rule_label TEXT").execute(&pool).await;

    // User long-term memory for AI personalization
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_memories (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            content TEXT NOT NULL,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );"
    )
    .execute(&pool)
    .await?;

    // Mail server error log (SMTP, AI, parsing failures)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS mail_errors (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            level TEXT NOT NULL DEFAULT 'ERROR',
            error_type TEXT NOT NULL,
            message TEXT NOT NULL,
            context TEXT,
            user_id TEXT,
            occurred_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );"
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE mail_errors ADD COLUMN level TEXT NOT NULL DEFAULT 'ERROR'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE mail_errors ADD COLUMN user_id TEXT").execute(&pool).await;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS data_deletion_requests (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            status TEXT NOT NULL DEFAULT 'requested',
            snapshot_json TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            email_confirmed_at DATETIME,
            finalized_at DATETIME,
            FOREIGN KEY(user_id) REFERENCES users(id)
        );"
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE data_deletion_requests ADD COLUMN email_confirmed_at DATETIME").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE data_deletion_requests ADD COLUMN finalized_at DATETIME").execute(&pool).await;

    // Auto-generated email replies based on rules (drafts or sent)
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS auto_replies (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            source_email_id TEXT,
            email_rule_id INTEGER NOT NULL,
            original_from TEXT NOT NULL,
            original_subject TEXT NOT NULL,
            original_received_at DATETIME,
            reply_generation_prompt TEXT,
            reply_body TEXT NOT NULL,
            reply_status TEXT NOT NULL DEFAULT 'draft',
            sent_at DATETIME,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id),
            FOREIGN KEY(email_rule_id) REFERENCES email_rules(id)
        );"
    )
    .execute(&pool)
    .await?;
    let _ = sqlx::query("ALTER TABLE auto_replies ADD COLUMN source_email_id TEXT").execute(&pool).await;

    // AI-extracted financial entries from decoded emails.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS email_financial_records (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            email_id TEXT NOT NULL,
            subject TEXT,
            reason TEXT NOT NULL,
            category TEXT NOT NULL,
            direction TEXT NOT NULL,
            amount REAL NOT NULL,
            currency TEXT NOT NULL DEFAULT 'TWD',
            month_key TEXT NOT NULL,
            month_total_after REAL NOT NULL DEFAULT 0,
            finance_type TEXT,
            due_date TEXT,
            statement_amount REAL,
            issuing_bank TEXT,
            card_last4 TEXT,
            transaction_month_key TEXT,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id),
            FOREIGN KEY(email_id) REFERENCES emails(id)
        );"
    )
    .execute(&pool)
    .await?;

    // Monthly aggregate totals by category.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS monthly_finance_summary (
            user_id TEXT NOT NULL,
            month_key TEXT NOT NULL,
            category TEXT NOT NULL,
            total_amount REAL NOT NULL DEFAULT 0,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY(user_id, month_key, category),
            FOREIGN KEY(user_id) REFERENCES users(id)
        );"
    )
    .execute(&pool)
    .await?;

    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN month_total_after REAL NOT NULL DEFAULT 0").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN currency TEXT NOT NULL DEFAULT 'TWD'").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN finance_type TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN due_date TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN statement_amount REAL").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN issuing_bank TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN card_last4 TEXT").execute(&pool).await;
    let _ = sqlx::query("ALTER TABLE email_financial_records ADD COLUMN transaction_month_key TEXT").execute(&pool).await;

    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_email_financial_records_user_created ON email_financial_records(user_id, created_at DESC)").execute(&pool).await;
    let _ = sqlx::query("CREATE INDEX IF NOT EXISTS idx_email_financial_records_user_email ON email_financial_records(user_id, email_id)").execute(&pool).await;

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
