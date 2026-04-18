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
}

#[derive(Serialize, FromRow)]
pub struct EmailRecord {
    pub id: String,
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub status: String,
    pub received_at: Option<String>,
}
