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
    #[serde(default = "default_mail_send_method")]
    pub mail_send_method: String,
    #[serde(default = "default_rule_label_mode")]
    pub rule_label_mode: String,
}

fn default_both() -> String { "both".to_string() }

fn default_true() -> bool { true }

fn default_utc() -> String { "UTC".to_string() }

fn default_language() -> String { "en".to_string() }

fn default_mail_send_method() -> String { "direct_mx".to_string() }

fn default_rule_label_mode() -> String { "ai_first".to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_serializes_and_deserializes() {
        #[derive(Serialize, Deserialize)]
        struct TestUser {
            #[serde(default = "default_true")]
            dry_run: bool,
        }

        fn default_true() -> bool { true }

        let json = r#"{"dry_run":false}"#;
        let user: TestUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.dry_run, false);

        let json2 = r#"{}"#;
        let user2: TestUser = serde_json::from_str(json2).unwrap();
        assert_eq!(user2.dry_run, true);
    }

    #[test]
    fn email_record_serialization() {
        let record = EmailRecord {
            id: "id1".to_string(),
            subject: Some("Test".to_string()),
            preview: Some("Preview".to_string()),
            status: "pending".to_string(),
            matched_rule_label: Some("label".to_string()),
            received_at: Some("2024-01-01".to_string()),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("id1"));
        assert!(json.contains("Test"));
    }
}

#[derive(Serialize, FromRow)]
pub struct EmailRecord {
    pub id: String,
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub status: String,
    pub matched_rule_label: Option<String>,
    pub received_at: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct ConsentAuditTrail {
    pub id: String,
    pub user_id: String,
    pub policy_version: String,
    pub consent_type: String,
    pub consent_granted: bool,
    pub consent_source: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, FromRow)]
pub struct DsarRequest {
    pub id: String,
    pub user_id: String,
    pub request_type: String,
    pub status: String,
    pub admin_notes: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, FromRow)]
pub struct DataRetentionPolicy {
    pub id: String,
    pub data_type: String,
    pub retention_days: i32,
    pub is_active: bool,
    pub updated_at: Option<String>,
}

#[derive(Serialize, FromRow)]
pub struct UserPrivacySettings {
    pub user_id: String,
    pub do_not_sell_share: bool,
    pub cross_border_disclosure_given: bool,
    pub data_location_preference: Option<String>,
    pub updated_at: String,
}

#[derive(Serialize, FromRow)]
pub struct UserAgeVerification {
    pub user_id: String,
    pub is_minor: bool,
    pub guardian_consent_given: bool,
    pub guardian_email: Option<String>,
    pub age_verified_at: Option<String>,
    pub created_at: String,
}

/// A feature wish that registered users can vote on.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeatureWish {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub is_official: bool,
    pub created_at: String,
    /// Computed at query time — total number of votes.
    pub vote_count: i64,
    /// Computed at query time — whether the requesting user has voted.
    pub user_has_voted: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateWishRequest {
    pub email: String,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VoteWishRequest {
    pub email: String,
}
