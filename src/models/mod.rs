use sqlx::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub is_onboarded: bool,
    pub preferences: Option<String>,
    pub magic_token: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub auto_reply: bool,
    #[serde(default = "default_true")]
    pub dry_run: bool,
    #[serde(default = "default_both")]
    pub email_format: String,
    pub display_name: Option<String>,
}

fn default_both() -> String { "both".to_string() }

fn default_true() -> bool { true }

#[derive(Serialize, FromRow)]
pub struct EmailRecord {
    pub id: String,
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub status: String,
    pub received_at: Option<String>,
}
