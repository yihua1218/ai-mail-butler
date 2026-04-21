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
    pub assistant_name_zh: Option<String>,
    pub assistant_name_en: Option<String>,
    pub assistant_tone_zh: Option<String>,
    pub assistant_tone_en: Option<String>,
    #[serde(default)]
    pub onboarding_step: i32,
    pub pdf_passwords: Option<String>,
    #[serde(default = "default_utc")]
    pub timezone: String,
    #[serde(default = "default_language")]
    pub preferred_language: String,
    #[serde(default)]
    pub training_data_consent: bool,
    pub training_consent_updated_at: Option<String>,
}

fn default_both() -> String { "both".to_string() }

fn default_true() -> bool { true }

fn default_utc() -> String { "UTC".to_string() }

fn default_language() -> String { "en".to_string() }

#[derive(Serialize, FromRow)]
pub struct EmailRecord {
    pub id: String,
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub status: String,
    pub matched_rule_label: Option<String>,
    pub received_at: Option<String>,
}
