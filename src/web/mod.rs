use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;
use std::path::PathBuf;
use tracing::info;
use anyhow::Result;
use tokio::fs;

use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::models::{User, EmailRecord};
use crate::ai::AiClient;
use crate::services::OnboardingService;

#[derive(sqlx::FromRow, Serialize)]
struct MailError {
    id: i64,
    level: String,
    error_type: String,
    message: String,
    context: Option<String>,
    user_id: Option<String>,
    user_email: Option<String>,
    occurred_at: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct EmailRule {
    id: i64,
    user_id: String,
    rule_text: String,
    source: String,
    is_enabled: bool,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
struct ChatFeedback {
    id: i64,
    user_email: Option<String>,
    ai_reply: String,
    rating: String,
    suggestion: Option<String>,
    is_read: bool,
    read_at: Option<String>,
    admin_reply: Option<String>,
    replied_at: Option<String>,
    replied_by: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct TrainingExportRow {
    id: i64,
    user_email: Option<String>,
    user_message: String,
    ai_reply: String,
    created_at: String,
}

fn parse_training_consent_answer(message: &str) -> Option<bool> {
    let m = message.trim().to_lowercase();
    let yes_markers = [
        "yes", "agree", "i agree", "consent", "allow", "ok",
        "同意", "願意", "可以", "好", "是",
    ];
    let no_markers = [
        "no", "disagree", "do not", "don't", "deny",
        "不同意", "不願意", "不要", "否", "不行",
    ];

    if no_markers.iter().any(|k| m.contains(k)) {
        return Some(false);
    }
    if yes_markers.iter().any(|k| m.contains(k)) {
        return Some(true);
    }
    None
}

fn redact_training_text(input: &str) -> String {
    static EMAIL_RE: OnceLock<Regex> = OnceLock::new();
    static US_PHONE_RE: OnceLock<Regex> = OnceLock::new();
    static TW_PHONE_RE: OnceLock<Regex> = OnceLock::new();
    static LONG_TOKEN_RE: OnceLock<Regex> = OnceLock::new();

    let email_re = EMAIL_RE.get_or_init(|| Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email regex"));
    let us_phone_re = US_PHONE_RE.get_or_init(|| Regex::new(r"(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)\d{3}[-.\s]?\d{4}").expect("us phone regex"));
    let tw_phone_re = TW_PHONE_RE.get_or_init(|| Regex::new(r"(?:\+886[-.\s]?)?0?9\d{2}[-.\s]?\d{3}[-.\s]?\d{3}").expect("tw phone regex"));
    let long_token_re = LONG_TOKEN_RE.get_or_init(|| Regex::new(r"\b[A-Za-z0-9_-]{24,}\b").expect("token regex"));

    let out = email_re.replace_all(input, "[REDACTED_EMAIL]").to_string();
    let out = us_phone_re.replace_all(&out, "[REDACTED_PHONE]").to_string();
    let out = tw_phone_re.replace_all(&out, "[REDACTED_PHONE]").to_string();
    long_token_re.replace_all(&out, "[REDACTED_TOKEN]").to_string()
}

#[derive(Clone, Default)]
struct DocsIndexEntry {
    file_name: String,
    content: String,
    lower_content: String,
}

#[derive(Default)]
struct DocsIndexCache {
    entries: Vec<DocsIndexEntry>,
    last_built_at: Option<Instant>,
}

static DOCS_INDEX_CACHE: OnceLock<RwLock<DocsIndexCache>> = OnceLock::new();
const DOCS_INDEX_TTL: Duration = Duration::from_secs(300);

fn docs_index_cache() -> &'static RwLock<DocsIndexCache> {
    DOCS_INDEX_CACHE.get_or_init(|| RwLock::new(DocsIndexCache::default()))
}

fn is_doc_allowed(file_name: &str, whitelist: &[String]) -> bool {
    if whitelist.is_empty() {
        return true;
    }

    let lowered_name = file_name.to_lowercase();
    whitelist.iter().any(|allowed| {
        let token = allowed.trim().to_lowercase();
        if token.is_empty() {
            return false;
        }
        lowered_name == token || lowered_name.ends_with(&token) || lowered_name.contains(&token)
    })
}

fn is_zh_tw_doc(file_name: &str) -> bool {
    file_name.to_lowercase().ends_with(".zh-tw.md")
}

fn language_bonus(file_name: &str, preferred_language: Option<&str>) -> usize {
    match preferred_language {
        Some("zh-TW") if is_zh_tw_doc(file_name) => 8,
        Some("zh-TW") => 1,
        Some(_) if is_zh_tw_doc(file_name) => 0,
        Some(_) => 4,
        None => 0,
    }
}

async fn rebuild_docs_index() -> DocsIndexCache {
    let mut entries = match fs::read_dir("docs").await {
        Ok(v) => v,
        Err(_) => {
            return DocsIndexCache {
                entries: Vec::new(),
                last_built_at: Some(Instant::now()),
            }
        }
    };

    let mut indexed: Vec<DocsIndexEntry> = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if ext != "md" {
            continue;
        }

        let Ok(content) = fs::read_to_string(&path).await else {
            continue;
        };
        let file_name = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("unknown.md")
            .to_string();
        indexed.push(DocsIndexEntry {
            file_name,
            lower_content: content.to_lowercase(),
            content,
        });
    }

    DocsIndexCache {
        entries: indexed,
        last_built_at: Some(Instant::now()),
    }
}

async fn ensure_docs_index_fresh() {
    let needs_rebuild = {
        let cache = docs_index_cache().read().await;
        cache.entries.is_empty()
            || cache
                .last_built_at
                .map(|t| t.elapsed() >= DOCS_INDEX_TTL)
                .unwrap_or(true)
    };
    if !needs_rebuild {
        return;
    }

    let new_cache = rebuild_docs_index().await;
    let mut cache = docs_index_cache().write().await;
    *cache = new_cache;
}

fn extract_query_terms(query: &str) -> Vec<String> {
    let lower = query.trim().to_lowercase();
    if lower.is_empty() {
        return Vec::new();
    }

    let mut terms: Vec<String> = lower
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '@')
        .filter(|t| t.len() >= 3)
        .map(|t| t.to_string())
        .collect();

    terms.push(lower);
    terms.sort();
    terms.dedup();
    terms
}

fn best_matching_snippet(content: &str, terms: &[String]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return String::new();
    }

    for (i, line) in lines.iter().enumerate() {
        let line_lower = line.to_lowercase();
        if terms.iter().any(|t| line_lower.contains(t)) {
            let start = i.saturating_sub(2);
            let end = (i + 4).min(lines.len().saturating_sub(1));
            return lines[start..=end].join("\n");
        }
    }

    lines.iter().take(6).cloned().collect::<Vec<_>>().join("\n")
}

async fn build_docs_context(query: &str, preferred_language: Option<&str>, docs_whitelist: &[String]) -> Option<String> {
    let terms = extract_query_terms(query);
    if terms.is_empty() {
        return None;
    }

    ensure_docs_index_fresh().await;
    let docs_entries = {
        let cache = docs_index_cache().read().await;
        cache.entries.clone()
    };
    if docs_entries.is_empty() {
        return None;
    }

    let mut scored: Vec<(usize, String, String)> = Vec::new();

    for entry in docs_entries.iter() {
        if !is_doc_allowed(&entry.file_name, docs_whitelist) {
            continue;
        }

        let score: usize = terms.iter().map(|t| entry.lower_content.matches(t).count()).sum();
        if score == 0 {
            continue;
        }

        let snippet = best_matching_snippet(&entry.content, &terms);
        let final_score = score + language_bonus(&entry.file_name, preferred_language);
        scored.push((final_score, entry.file_name.clone(), snippet));
    }

    if scored.is_empty() {
        return None;
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let context = scored
        .into_iter()
        .take(3)
        .map(|(_, file, snippet)| format!("[Source: {}]\n{}", file, snippet))
        .collect::<Vec<_>>()
        .join("\n\n");

    Some(context)
}

async fn log_mail_event(
    pool: &SqlitePool,
    level: &str,
    error_type: &str,
    msg: &str,
    context: Option<&str>,
    user_id: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO mail_errors (level, error_type, message, context, user_id) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(level)
    .bind(error_type)
    .bind(msg)
    .bind(context)
    .bind(user_id)
    .execute(pool)
    .await;
}

fn looks_like_email_rule(message: &str) -> bool {
    let m = message.trim();
    if m.len() < 8 {
        return false;
    }

    let lower = m.to_lowercase();
    let keywords = [
        "轉寄", "請幫我", "遇到", "收到", "帳單", "通知", "發票", "提醒", "回覆", "處理", "規則", "新增規則", "建立規則",
        "forward", "when", "invoice", "bill", "receipt", "notify", "remind", "reply", "urgent", "important", "rule", "add rule", "create rule", "set a rule",
    ];
    keywords.iter().any(|k| lower.contains(k))
}

enum RuleCaptureOutcome {
    None,
    Duplicate(String),
    Created(String),
}

fn strip_leading_rule_prefix(message: &str, lower: &str, prefix: &str) -> Option<String> {
    if !lower.starts_with(prefix) {
        return None;
    }

    let trimmed = message
        .chars()
        .skip(prefix.chars().count())
        .collect::<String>()
        .trim()
        .trim_start_matches([':', '：', '-', ' '])
        .trim()
        .to_string();

    if trimmed.len() >= 6 {
        Some(trimmed)
    } else {
        None
    }
}

fn extract_rule_from_message(message: &str) -> Option<String> {
    let cleaned = message.trim();
    if cleaned.len() < 8 {
        return None;
    }

    let lower = cleaned.to_lowercase();
    let prefixes = [
        "新增規則", "建立規則", "設定規則", "幫我新增規則", "幫我建立規則",
        "add rule", "create rule", "set a rule", "new rule", "rule",
    ];

    for p in prefixes {
        if let Some(value) = strip_leading_rule_prefix(cleaned, &lower, p) {
            return Some(value);
        }
    }

    if (lower.starts_with("規則:") || lower.starts_with("規則：")) && cleaned.len() > 3 {
        let extracted = cleaned
            .chars()
            .skip(3)
            .collect::<String>()
            .trim()
            .to_string();
        if extracted.len() >= 6 {
            return Some(extracted);
        }
    }

    if looks_like_email_rule(cleaned) {
        return Some(cleaned.to_string());
    }

    None
}

fn rule_capture_notice(preferred_language: &str, outcome: &RuleCaptureOutcome) -> Option<String> {
    match outcome {
        RuleCaptureOutcome::Created(rule) => {
            if preferred_language == "zh-TW" {
                Some(format!("[Rule Created] 已根據你的需求建立新規則：{}", rule))
            } else {
                Some(format!("[Rule Created] I created a new rule from your request: {}", rule))
            }
        }
        RuleCaptureOutcome::Duplicate(rule) => {
            if preferred_language == "zh-TW" {
                Some(format!("[Rule Exists] 這條規則已存在，已略過重複新增：{}", rule))
            } else {
                Some(format!("[Rule Exists] This rule already exists, so I skipped creating a duplicate: {}", rule))
            }
        }
        RuleCaptureOutcome::None => None,
    }
}

async fn capture_rule_from_chat(pool: &SqlitePool, user_id: &str, message: &str) -> RuleCaptureOutcome {
    let Some(cleaned) = extract_rule_from_message(message) else {
        return RuleCaptureOutcome::None;
    };

    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM email_rules WHERE user_id = ? AND lower(rule_text) = lower(?) LIMIT 1"
    )
    .bind(user_id)
    .bind(&cleaned)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if exists.is_some() {
        return RuleCaptureOutcome::Duplicate(cleaned);
    }

    let insert_result = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, source, is_enabled) VALUES (?, ?, 'chat', 1)"
    )
    .bind(user_id)
    .bind(&cleaned)
    .execute(pool)
    .await;

    if insert_result.is_ok() {
        RuleCaptureOutcome::Created(cleaned)
    } else {
        RuleCaptureOutcome::None
    }
}

async fn get_user_id_by_email(pool: &SqlitePool, email: &str) -> Option<String> {
    sqlx::query_scalar("SELECT id FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct DataDeletionSnapshot {
    email_count: i64,
    rule_count: i64,
    log_count: i64,
    memory_count: i64,
    activity_row_count: i64,
    activity_event_total: i64,
    chat_log_count: i64,
    file_count: i64,
    total_file_bytes: i64,
}

#[derive(sqlx::FromRow)]
struct DataDeletionRequestRow {
    id: String,
    user_id: String,
    user_email: String,
    status: String,
    snapshot_json: String,
}

fn sanitize_path_component(input: &str) -> String {
    let sanitized: String = input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();

    if sanitized.is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

async fn collect_dir_stats(base_dir: &str) -> (i64, i64) {
    let mut total_files = 0_i64;
    let mut total_bytes = 0_i64;
    let mut stack = vec![PathBuf::from(base_dir)];

    while let Some(dir) = stack.pop() {
        let mut entries = match fs::read_dir(&dir).await {
            Ok(v) => v,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(meta) = entry.metadata().await {
                if meta.is_dir() {
                    stack.push(entry.path());
                } else if meta.is_file() {
                    total_files += 1;
                    total_bytes += meta.len() as i64;
                }
            }
        }
    }

    (total_files, total_bytes)
}

async fn build_user_data_snapshot(pool: &SqlitePool, user_id: &str, user_email: &str) -> DataDeletionSnapshot {
    let sender_dir = format!("data/mail_spool/{}", sanitize_path_component(&user_email.to_ascii_lowercase()));
    let (file_count, total_file_bytes) = collect_dir_stats(&sender_dir).await;

    let email_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let rule_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mail_errors WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let memory_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_memories WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let activity_row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM user_activity_stats WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let activity_event_total: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(count), 0) FROM user_activity_stats WHERE user_id = ?")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let chat_log_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs WHERE user_email = ?")
        .bind(user_email)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    DataDeletionSnapshot {
        email_count,
        rule_count,
        log_count,
        memory_count,
        activity_row_count,
        activity_event_total,
        chat_log_count,
        file_count,
        total_file_bytes,
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub ai_client: AiClient,
    pub admin_email: Option<String>,
    pub developer_email: Option<String>,
    pub config: std::sync::Arc<crate::config::Config>,
}

fn is_admin_or_developer(state: &AppState, email: &str) -> bool {
    Some(email) == state.admin_email.as_deref() || Some(email) == state.developer_email.as_deref()
}

fn role_for_email(state: &AppState, email: &str) -> String {
    if Some(email) == state.admin_email.as_deref() {
        "admin".to_string()
    } else if Some(email) == state.developer_email.as_deref() {
        "developer".to_string()
    } else {
        "user".to_string()
    }
}

#[derive(Deserialize)]
struct AuthQuery {
    email: Option<String>,
}

async fn get_me(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<Option<User>> {
    if let Some(email) = query.email {
        if email.is_empty() { return Json(None); }
        
        // Use UPSERT logic or separate check to handle concurrency/re-registrations
        let role = role_for_email(&state, &email);
        let new_id = uuid::Uuid::new_v4().to_string();
        
        let _ = sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
            .bind(&new_id)
            .bind(&email)
            .execute(&state.pool).await;

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await.unwrap_or(None);
            
        if let Some(mut u) = user { 
            u.role = role;
            return Json(Some(u)); 
        }
    }
    Json(None)
}

async fn get_dashboard(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let pool = &state.pool;
    
    // Always fetch global stats
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await.unwrap_or(0);
    let received_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails").fetch_one(pool).await.unwrap_or(0);
    let replied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE status = 'replied'").fetch_one(pool).await.unwrap_or(0);
    let ai_replies_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs").fetch_one(pool).await.unwrap_or(0);
    
    let global_stats = serde_json::json!({
        "registered_users": users_count,
        "emails_received": received_count,
        "emails_replied": replied_count,
        "emails_sent": 0,
        "ai_replies": ai_replies_count,
    });
    
    if let Some(email) = query.email {
        if !email.is_empty() {
            let is_admin = is_admin_or_developer(&state, &email);
            
            let user_id: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
                .bind(&email)
                .fetch_optional(pool).await.unwrap_or(None);
                
            if let Some((uid,)) = user_id {
                let personal_emails = sqlx::query_as::<_, EmailRecord>("SELECT id, subject, preview, status, CAST(received_at AS TEXT) as received_at FROM emails WHERE user_id = ? ORDER BY received_at DESC")
                    .bind(&uid)
                    .fetch_all(pool).await.unwrap_or(vec![]);
                    
                let p_received: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ?").bind(&uid).fetch_one(pool).await.unwrap_or(0);
                let p_replied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ? AND status = 'replied'").bind(&uid).fetch_one(pool).await.unwrap_or(0);
                
                let personal_stats = serde_json::json!({
                    "emails_received": p_received,
                    "emails_replied": p_replied,
                });

                if is_admin {
                    return Json(serde_json::json!({ 
                        "type": "admin", 
                        "global_stats": global_stats,
                        "personal_stats": personal_stats,
                        "personal_emails": personal_emails
                    }));
                } else {
                    return Json(serde_json::json!({ 
                        "type": "personal", 
                        "global_stats": global_stats,
                        "personal_stats": personal_stats,
                        "personal_emails": personal_emails
                    }));
                }
            }
        }
    }
    
    // Anonymous
    Json(serde_json::json!({
        "type": "anonymous",
        "global_stats": global_stats
    }))
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    email: String,
    guest_name: Option<String>,
}

#[derive(Deserialize)]
struct ChatFeedbackRequest {
    email: Option<String>,
    ai_reply: String,
    rating: String,
    suggestion: Option<String>,
}

async fn post_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email;
    let message = payload.message;
    let guest_name = payload.guest_name;
    let pool = &state.pool;

    let chat_res = if !email.is_empty() {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(pool).await.unwrap_or(None);

        if let Some(mut user) = user {
            if user.onboarding_step == 0 {
                if let Some(consent) = parse_training_consent_answer(&message) {
                    let _ = sqlx::query("UPDATE users SET training_data_consent = ?, training_consent_updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                        .bind(consent)
                        .bind(&user.id)
                        .execute(pool)
                        .await;
                    user.training_data_consent = consent;
                }
            }

            // Update preferences
            let new_pref = match OnboardingService::extract_preferences(&state.ai_client, &user, &message).await {
                Ok(p) => p,
                Err(_) => user.preferences.clone().unwrap_or_default()
            };
            sqlx::query("UPDATE users SET preferences = ?, is_onboarded = true WHERE id = ?")
                .bind(&new_pref).bind(&user.id).execute(pool).await.ok();
            user.preferences = Some(new_pref);

            let docs_context = build_docs_context(
                &message,
                Some(user.preferred_language.as_str()),
                &state.config.docs_whitelist,
            ).await;

            let memory = OnboardingService::get_memory(pool, &user.id).await;
            let mut res = match OnboardingService::generate_reply(&state.ai_client, &user, &message, &memory, &state.config.assistant_email, None, docs_context.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to generate reply: {}", e);
                    crate::ai::ChatResult { content: "Sorry, I am having trouble connecting to my AI brain.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None }
                }
            };

            // Detect repetitive questions (simple keyword for now)
            let lower_msg = message.to_lowercase();
            if lower_msg.contains("forward") || lower_msg.contains("email") || lower_msg.contains("信箱") || lower_msg.contains("轉寄") {
                let _ = OnboardingService::log_activity(pool, &user.id, "ask_forwarding_info").await;
            }

            let rule_capture = capture_rule_from_chat(pool, &user.id, &message).await;
            if let Some(notice) = rule_capture_notice(&user.preferred_language, &rule_capture) {
                res.content = format!("{}\n\n{}", res.content, notice);
            }

            // Append Onboarding Question if needed
            if user.onboarding_step < 4 {
                if let Some(question) = OnboardingService::get_next_onboarding_question(&user).await {
                    res.content = format!("{}\n\n---\n💡 [Onboarding] {}", res.content, question);
                }
                // Advance onboarding step
                sqlx::query("UPDATE users SET onboarding_step = onboarding_step + 1 WHERE id = ?")
                    .bind(&user.id).execute(pool).await.ok();
            }

            let ai_client_clone = state.ai_client.clone();
            let pool_clone = state.pool.clone();
            let user_id = user.id.clone();
            let msg_clone = message.clone();
            let reply_clone = res.content.clone();
            tokio::spawn(async move {
                let _ = OnboardingService::update_memory(&ai_client_clone, &pool_clone, &user_id, &msg_clone, &reply_clone).await;
            });

            let _ = sqlx::query(
                "INSERT INTO chat_transcripts (user_id, user_email, user_message, ai_reply) VALUES (?, ?, ?, ?)"
            )
            .bind(&user.id)
            .bind(&user.email)
            .bind(&message)
            .bind(&res.content)
            .execute(pool)
            .await;

            res
        } else {
            let docs_context = build_docs_context(&message, None, &state.config.docs_whitelist).await;
            OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name, &state.config.assistant_email, docs_context.clone()).await
                .unwrap_or_else(|_| crate::ai::ChatResult { content: "Error connecting to AI.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None })
        }
    } else {
        let docs_context = build_docs_context(&message, None, &state.config.docs_whitelist).await;
        OnboardingService::generate_anonymous_reply(&state.ai_client, &message, guest_name, &state.config.assistant_email, docs_context.clone()).await
            .unwrap_or_else(|_| crate::ai::ChatResult { content: "Error connecting to AI.".to_string(), total_tokens: 0, duration_ms: 0, finish_reason: None })
    };

    // Record this AI reply in chat_logs
    let log_email = if email.is_empty() { None } else { Some(email.clone()) };
    sqlx::query("INSERT INTO chat_logs (user_email) VALUES (?)")
        .bind(&log_email).execute(pool).await.ok();

    Json(serde_json::json!({ 
        "reply": chat_res.content, 
        "total_tokens": chat_res.total_tokens,
        "duration_ms": chat_res.duration_ms,
        "finish_reason": chat_res.finish_reason,
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

async fn get_training_export(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };
    if !is_admin_or_developer(&state, &email) {
        return Json(serde_json::json!({ "status": "error", "message": "Unauthorized" }));
    }

    let rows = sqlx::query_as::<_, TrainingExportRow>(
        "SELECT t.id, t.user_email, t.user_message, t.ai_reply, CAST(t.created_at AS TEXT) as created_at \
         FROM chat_transcripts t \
         JOIN users u ON u.id = t.user_id \
         WHERE u.training_data_consent = 1 \
         ORDER BY t.created_at DESC LIMIT 2000"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let sanitized: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "user_email": r.user_email.as_deref().map(redact_training_text),
                "user_message": redact_training_text(&r.user_message),
                "ai_reply": redact_training_text(&r.ai_reply),
                "created_at": r.created_at,
            })
        })
        .collect();

    Json(serde_json::json!({
        "status": "success",
        "records": sanitized,
        "note": "Only consented users are exported and all records are de-identified.",
    }))
}

async fn post_chat_feedback(
    State(state): State<AppState>,
    Json(payload): Json<ChatFeedbackRequest>,
) -> Json<serde_json::Value> {
    let rating = payload.rating.trim().to_ascii_lowercase();
    if rating != "up" && rating != "down" {
        return Json(serde_json::json!({ "status": "error", "message": "Invalid rating" }));
    }

    let ai_reply = payload.ai_reply.trim();
    if ai_reply.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Missing ai_reply" }));
    }

    let normalized_email = payload
        .email
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_ascii_lowercase());
    let suggestion = payload
        .suggestion
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());

    let result = sqlx::query(
        "INSERT INTO chat_feedback (user_email, ai_reply, rating, suggestion, is_read) VALUES (?, ?, ?, ?, 0)"
    )
    .bind(normalized_email.clone())
    .bind(ai_reply)
    .bind(rating)
    .bind(suggestion)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => {
            if let Some(admin_email) = state.admin_email.clone() {
                let preview = if ai_reply.chars().count() > 240 {
                    format!("{}...", ai_reply.chars().take(240).collect::<String>())
                } else {
                    ai_reply.to_string()
                };
                let suggestion_text = payload.suggestion
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .unwrap_or("(none)");
                let sender = normalized_email.as_deref().unwrap_or("anonymous");
                let subject = format!("[AI Mail Butler] New feedback from {}", sender);
                let text_body = format!(
                    "A new feedback was submitted.\n\nFrom: {}\nRating: {}\nSuggestion: {}\n\nAI Reply Preview:\n{}\n",
                    sender,
                    payload.rating,
                    suggestion_text,
                    preview
                );
                let html_body = format!(
                    "<h3>New feedback submitted</h3><p><strong>From:</strong> {}</p><p><strong>Rating:</strong> {}</p><p><strong>Suggestion:</strong> {}</p><p><strong>AI Reply Preview:</strong><br/>{}</p>",
                    sender,
                    payload.rating,
                    suggestion_text,
                    preview.replace('\n', "<br/>")
                );
                let _ = send_system_email_as_assistant(&state, &admin_email, &subject, &text_body, &html_body).await;
            }
            Json(serde_json::json!({ "status": "success" }))
        }
        Err(e) => {
            tracing::error!("Failed to save chat feedback: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Save failed" }))
        }
    }
}

async fn get_feedback(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };
    if email.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    }

    let is_privileged = is_admin_or_developer(&state, &email);
    let feedback = if is_privileged {
        sqlx::query_as::<_, ChatFeedback>(
            "SELECT id, user_email, ai_reply, rating, suggestion, is_read, CAST(read_at AS TEXT) as read_at, admin_reply, CAST(replied_at AS TEXT) as replied_at, replied_by, CAST(created_at AS TEXT) as created_at \
             FROM chat_feedback ORDER BY created_at DESC LIMIT 500"
        )
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    } else {
        sqlx::query_as::<_, ChatFeedback>(
            "SELECT id, user_email, ai_reply, rating, suggestion, is_read, CAST(read_at AS TEXT) as read_at, admin_reply, CAST(replied_at AS TEXT) as replied_at, replied_by, CAST(created_at AS TEXT) as created_at \
             FROM chat_feedback WHERE lower(user_email) = lower(?) ORDER BY created_at DESC LIMIT 500"
        )
        .bind(email)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default()
    };

    Json(serde_json::json!({ "status": "success", "feedback": feedback }))
}

#[derive(Deserialize)]
struct MarkFeedbackReadRequest {
    email: String,
    feedback_id: i64,
    is_read: bool,
}

async fn post_mark_feedback_read(
    State(state): State<AppState>,
    Json(payload): Json<MarkFeedbackReadRequest>,
) -> Json<serde_json::Value> {
    if !is_admin_or_developer(&state, &payload.email) {
        return Json(serde_json::json!({ "status": "error", "message": "Unauthorized" }));
    }

    let result = if payload.is_read {
        sqlx::query("UPDATE chat_feedback SET is_read = 1, read_at = COALESCE(read_at, CURRENT_TIMESTAMP) WHERE id = ?")
            .bind(payload.feedback_id)
            .execute(&state.pool)
            .await
    } else {
        sqlx::query("UPDATE chat_feedback SET is_read = 0, read_at = NULL WHERE id = ?")
            .bind(payload.feedback_id)
            .execute(&state.pool)
            .await
    };

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update feedback read state: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Update failed" }))
        }
    }
}

#[derive(Deserialize)]
struct ReplyFeedbackRequest {
    email: String,
    feedback_id: i64,
    reply_message: String,
}

async fn post_reply_feedback(
    State(state): State<AppState>,
    Json(payload): Json<ReplyFeedbackRequest>,
) -> Json<serde_json::Value> {
    if !is_admin_or_developer(&state, &payload.email) {
        return Json(serde_json::json!({ "status": "error", "message": "Unauthorized" }));
    }

    let reply_message = payload.reply_message.trim();
    if reply_message.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Reply cannot be empty" }));
    }

    let feedback_target: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT user_email, suggestion FROM chat_feedback WHERE id = ?"
    )
    .bind(payload.feedback_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some((user_email_opt, suggestion_opt)) = feedback_target else {
        return Json(serde_json::json!({ "status": "error", "message": "Feedback not found" }));
    };

    let result = sqlx::query(
        "UPDATE chat_feedback SET admin_reply = ?, replied_at = CURRENT_TIMESTAMP, replied_by = ?, is_read = 1, read_at = COALESCE(read_at, CURRENT_TIMESTAMP) WHERE id = ?"
    )
    .bind(reply_message)
    .bind(&payload.email)
    .bind(payload.feedback_id)
    .execute(&state.pool)
    .await;

    if let Err(e) = result {
        tracing::error!("Failed to save feedback reply: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": "Reply save failed" }));
    }

    if let Some(user_email) = user_email_opt {
        let subject = "AI 助理已回覆你的建議 / AI Assistant Reply to Your Feedback";
        let suggestion = suggestion_opt.unwrap_or_else(|| "(none)".to_string());
        let text_body = format!(
            "你好，\n\n我們已收到你提供的回饋，AI 助理回覆如下：\n\n{}\n\n你的原始建議：{}\n\n感謝你的協助！",
            reply_message,
            suggestion
        );
        let html_body = format!(
            "<p>你好，</p><p>我們已收到你提供的回饋，AI 助理回覆如下：</p><blockquote>{}</blockquote><p><strong>你的原始建議：</strong> {}</p><p>感謝你的協助！</p>",
            reply_message.replace('\n', "<br/>"),
            suggestion.replace('\n', "<br/>")
        );
        let _ = send_system_email_as_assistant(&state, &user_email, subject, &text_body, &html_body).await;
    }

    Json(serde_json::json!({ "status": "success" }))
}

#[derive(Serialize)]
struct BuildInfo {
    version: &'static str,
    target: &'static str,
    host: &'static str,
    profile: &'static str,
    git_commit: &'static str,
    build_date: &'static str,
    // Build-machine hardware fingerprint
    build_cpu_cores: &'static str,
    build_cpu_model: &'static str,
    build_ram: &'static str,
    build_disk: &'static str,
    assistant_email: String,
}

async fn get_about(State(state): State<AppState>) -> Json<BuildInfo> {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        target: env!("BUILD_TARGET"),
        host: env!("BUILD_HOST"),
        profile: env!("BUILD_PROFILE"),
        git_commit: env!("GIT_COMMIT"),
        build_date: env!("BUILD_DATE"),
        build_cpu_cores: env!("BUILD_CPU_CORES"),
        build_cpu_model: env!("BUILD_CPU_MODEL"),
        build_ram: env!("BUILD_RAM"),
        build_disk: env!("BUILD_DISK"),
        assistant_email: state.config.assistant_email.clone(),
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

use mail_send::{SmtpClientBuilder, mail_builder::MessageBuilder};
use lettre::message::{Message, SinglePart};
use lettre::{SmtpTransport, Transport};

async fn send_system_email_as_assistant(
    state: &AppState,
    to_email: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> bool {
    let smtp_ready = state.config.smtp_relay_host.as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);
    if !smtp_ready {
        tracing::warn!("SMTP relay is not configured, skip sending email to {}", to_email);
        return false;
    }

    let host = state.config.smtp_relay_host.clone().unwrap_or_default();
    let port = state.config.smtp_relay_port;
    let smtp_user = state.config.smtp_relay_user.clone().unwrap_or_default();
    let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();
    let assistant_name = "AI 助理";
    let assistant_mailbox = format!("{} <{}>", assistant_name, state.config.assistant_email);

    let message = MessageBuilder::new()
        .from(assistant_mailbox.clone())
        .reply_to(assistant_mailbox)
        .to(to_email.to_string())
        .subject(subject)
        .text_body(text_body.to_string())
        .html_body(html_body.to_string());

    let is_implicit = port == 465;
    let mut builder = SmtpClientBuilder::new(host.as_str(), port).implicit_tls(is_implicit);
    if !smtp_user.is_empty() {
        builder = builder.credentials((smtp_user.as_str(), pass.as_str()));
    }

    match builder.connect().await {
        Ok(mut client) => client.send(message).await.is_ok(),
        Err(e) => {
            tracing::error!("Failed to connect SMTP for system email: {:?}", e);
            false
        }
    }
}


async fn post_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email.trim().to_lowercase();
    let token = uuid::Uuid::new_v4().to_string();

    // Use a true UPSERT to avoid UNIQUE constraint race conditions:
    // If email already exists, just update the token; otherwise insert.
    let new_id = uuid::Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO users (id, email, magic_token) VALUES (?, ?, ?)
         ON CONFLICT(email) DO UPDATE SET magic_token = excluded.magic_token"
    )
    .bind(&new_id)
    .bind(&email)
    .bind(&token)
    .execute(&state.pool).await;

    if let Err(e) = result {
        tracing::error!("DB error during magic link upsert: {:?}", e);
        return Json(serde_json::json!({ "status": "error", "message": "Internal server error" }));
    }

    // Determine the public base URL for the login link
    let base_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", state.config.server_port));
    let login_url = format!("{}/login?token={}", base_url, token);

    // ALWAYS log the magic link to terminal so it's usable even without SMTP
    let log_box = format!(
        "\n\
        ╔══════════════════════════════════════════════════════════════════════════════════════════╗\n\
        ║  MAGIC LOGIN LINK (debug)                                                                ║\n\
        ║  To : {:<82} ║\n\
        ║  URL: {:<82} ║\n\
        ╚══════════════════════════════════════════════════════════════════════════════════════════╝",
        email, login_url
    );
    tracing::info!("{}", log_box);

    // Check if SMTP is configured with real values (not placeholder)
    let smtp_ready = state.config.smtp_relay_host.as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);

    let from_addr: lettre::message::Mailbox = state.config.assistant_email.parse().unwrap_or_else(|_| "noreply@example.com".parse().unwrap());
    let to_addr: lettre::message::Mailbox = email.parse().unwrap_or_else(|_| "noreply@example.com".parse().unwrap());
    let mut subject = "Your AI Mail Butler Login Link".to_string();
    let mut plain_text = format!(
        "Welcome to AI Mail Butler!\n\nClick the link below to securely login without a password:\n{}",
        login_url
    );
    if smtp_ready {
        let host = state.config.smtp_relay_host.as_ref().unwrap();
        let port = state.config.smtp_relay_port;
        let user = state.config.smtp_relay_user.clone().unwrap_or_default();
        let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();

        let user_pref: (String, String) = sqlx::query_as("SELECT email_format, preferred_language FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await
            .unwrap_or(Some(("both".to_string(), "en".to_string())))
            .unwrap_or(("both".to_string(), "en".to_string()));
        let email_format = user_pref.0;
        let preferred_language = user_pref.1;

        if preferred_language == "zh-TW" {
            subject = "您的 AI 郵件助理登入連結".to_string();
            plain_text = format!(
                "歡迎使用 AI Mail Butler！\n\n請點擊下方連結安全無密碼登入：\n{}",
                login_url
            );
        }

        let assistant_name = if preferred_language == "zh-TW" {
            "AI 郵件助理"
        } else {
            "AI Mail Butler"
        };
        let assistant_mailbox = format!("{} <{}>", assistant_name, state.config.assistant_email);

        let html_body = if preferred_language == "zh-TW" {
            format!(
            r#"<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; padding: 40px; background-color: #f5f5f7; color: #1d1d1f;">
    <div style="max-width: 600px; margin: 0 auto; background-color: #ffffff; padding: 32px; border-radius: 20px; box-shadow: 0 4px 12px rgba(0,0,0,0.05);">
        <h1 style="font-size: 24px; font-weight: 600; margin-bottom: 16px;">AI Mail Butler</h1>
        <p style="font-size: 16px; line-height: 1.5; margin-bottom: 24px;">您好！請點擊下方的按鈕以登入您的 AI 郵件助理管理後台。</p>
        <div style="text-align: center; margin: 32px 0;">
            <a href="{}" style="display: inline-block; padding: 14px 32px; background-color: #007aff; color: #ffffff; text-decoration: none; border-radius: 12px; font-weight: 500; font-size: 16px;">登入管理後台</a>
        </div>
        <p style="font-size: 14px; color: #86868b; margin-top: 24px;">如果按鈕無法運作，請複製以下連結至瀏覽器：<br>
        <span style="word-break: break-all; color: #007aff;">{}</span></p>
        <hr style="border: none; border-top: 1px solid #d2d2d7; margin: 32px 0;">
        <p style="font-size: 12px; color: #86868b;">如果您沒有要求此連結，請忽略此郵件。為了您的帳號安全，請勿將此連結轉寄給他人。</p>
    </div>
</body>
</html>"#,
            login_url, login_url
            )
        } else {
            format!(
                r#"<!DOCTYPE html>
<html>
<body style="font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; padding: 40px; background-color: #f5f5f7; color: #1d1d1f;">
    <div style="max-width: 600px; margin: 0 auto; background-color: #ffffff; padding: 32px; border-radius: 20px; box-shadow: 0 4px 12px rgba(0,0,0,0.05);">
        <h1 style="font-size: 24px; font-weight: 600; margin-bottom: 16px;">AI Mail Butler</h1>
        <p style="font-size: 16px; line-height: 1.5; margin-bottom: 24px;">Hello! Click the button below to login to your AI Mail Butler dashboard.</p>
        <div style="text-align: center; margin: 32px 0;">
            <a href="{}" style="display: inline-block; padding: 14px 32px; background-color: #007aff; color: #ffffff; text-decoration: none; border-radius: 12px; font-weight: 500; font-size: 16px;">Login Dashboard</a>
        </div>
        <p style="font-size: 14px; color: #86868b; margin-top: 24px;">If the button does not work, copy this link in browser:<br>
        <span style="word-break: break-all; color: #007aff;">{}</span></p>
        <hr style="border: none; border-top: 1px solid #d2d2d7; margin: 32px 0;">
        <p style="font-size: 12px; color: #86868b;">If you did not request this link, please ignore this email.</p>
    </div>
</body>
</html>"#,
                login_url, login_url
            )
        };

        // Build message using mail-send's builder
        let mut builder = MessageBuilder::new()
            .from(assistant_mailbox.clone())
            .reply_to(assistant_mailbox)
            .to(to_addr.to_string())
            .subject(subject.as_str());

        match email_format.as_str() {
            "html" => { builder = builder.html_body(html_body); },
            "plain" => { builder = builder.text_body(plain_text.clone()); },
            _ => { 
                builder = builder.text_body(plain_text.clone()).html_body(html_body);
            }
        }
        
        let message = builder;

        let is_implicit = port == 465;
        tracing::debug!(">>> [SMTP] Connecting to {}:{} (Implicit TLS: {}, Format: {}, Lang: {})", host, port, is_implicit, email_format, preferred_language);

        let send_task = async move {
            let mut builder = SmtpClientBuilder::new(host.as_str(), port);
            builder = builder.implicit_tls(is_implicit);
            
            if !user.is_empty() {
                builder = builder.credentials((user.as_str(), pass.as_str()));
            }
            
            let mut client = builder.connect().await?;
            client.send(message).await
        };

        match send_task.await {
            Ok(_) => {
                tracing::info!("Magic link email sent successfully via mail-send to {}", email);
                Json(serde_json::json!({ "status": "success", "message": "Magic link sent to your email" }))
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                tracing::error!("mail-send delivery failed: {}. Login URL is in server console.", err_msg);
                let user_id = get_user_id_by_email(&state.pool, &email).await;
                log_mail_event(&state.pool, "ERROR", "smtp_send", &err_msg, Some(&email), user_id.as_deref()).await;
                Json(serde_json::json!({ "status": "ok_debug", "message": "Email delivery failed; login URL logged to console" }))
            }
        }
    } else {
        // No relay configured — try direct MX delivery to the recipient's mail server.
        // This may land in spam but it works without any relay setup.
        let domain = email.split('@').nth(1).unwrap_or("").to_string();
        let mx_host = lookup_mx_host(&domain).await;

        if let Some(mx) = mx_host {
            tracing::info!("No SMTP relay configured. Attempting direct MX delivery to {} ({})", mx, domain);
            match Message::builder().from(from_addr).to(to_addr).subject(subject.as_str()).singlepart(SinglePart::plain(plain_text)) {
                Ok(email_msg) => {
                    // Direct SMTP: connect to MX on port 25, STARTTLS if available
                    match SmtpTransport::relay(&mx) {
                        Ok(t_builder) => {
                            let mailer = t_builder.port(25).build();
                            match mailer.send(&email_msg) {
                                Ok(_) => {
                                    tracing::info!("Direct MX delivery succeeded to {} via {}", email, mx);
                                    return Json(serde_json::json!({ "status": "success", "message": "Magic link sent via direct delivery (may be in spam)" }));
                                }
                                Err(e) => {
                                    tracing::warn!("Direct MX delivery failed: {:?}. Login URL is in server console.", e);
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Could not build direct SMTP transport: {:?}", e);
                        }
                    }
                }
                Err(e) => tracing::warn!("Email build error for direct delivery: {:?}", e),
            }
        } else {
            tracing::warn!("Could not resolve MX for domain '{}'. Login URL is in server console.", domain);
        }

        Json(serde_json::json!({ "status": "ok_debug", "message": "No SMTP relay configured — login URL printed to server console" }))
    }
}

/// Look up the highest-priority MX record for a domain using `dig`.
/// Returns the MX hostname (without trailing dot) or None if lookup fails.
async fn lookup_mx_host(domain: &str) -> Option<String> {
    tracing::debug!("Looking up MX records for domain: {}", domain);
    // Try `dig +short MX {domain}` — available on Linux and macOS
    match tokio::process::Command::new("dig")
        .args(["+short", "MX", domain])
        .output()
        .await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().is_empty() {
                    tracing::warn!("dig returned empty results for MX lookup of {}", domain);
                    return None;
                }
                parse_mx_records(&stdout)
            },
            Err(e) => {
                tracing::error!("Failed to execute 'dig' command: {}. Make sure 'dnsutils' or 'bind9-host' is installed.", e);
                None
            }
        }
}

/// Parse `dig +short MX` output lines like "10 alt1.gmail-smtp-in.l.google.com."
/// Returns the hostname of the lowest-priority (most preferred) MX record.
fn parse_mx_records(output: &str) -> Option<String> {
    let mut records: Vec<(u32, String)> = output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.trim().splitn(2, ' ').collect();
            if parts.len() == 2 {
                let priority: u32 = parts[0].parse().ok()?;
                let host = parts[1].trim_end_matches('.').to_string();
                if !host.is_empty() { Some((priority, host)) } else { None }
            } else {
                None
            }
        })
        .collect();

    records.sort_by_key(|(p, _)| *p);
    records.into_iter().next().map(|(_, host)| host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_like_email_rule_detects_common_intents() {
        assert!(looks_like_email_rule("請幫我把帳單通知都轉寄並提醒我"));
        assert!(looks_like_email_rule("When I receive invoice emails, remind me to reply quickly"));
        assert!(looks_like_email_rule("Please add rule: remind me when invoice arrives"));
        assert!(!looks_like_email_rule("hi"));
        assert!(!looks_like_email_rule("just browsing this dashboard"));
    }

    #[test]
    fn extract_rule_from_message_parses_explicit_prefixes() {
        assert_eq!(
            extract_rule_from_message("新增規則：收到發票時提醒我回覆"),
            Some("收到發票時提醒我回覆".to_string())
        );
        assert_eq!(
            extract_rule_from_message("add rule: when invoice email arrives, remind me"),
            Some("when invoice email arrives, remind me".to_string())
        );
        assert_eq!(extract_rule_from_message("hello there"), None);
    }

    #[test]
    fn docs_language_bonus_and_allowlist_behave_as_expected() {
        assert!(is_doc_allowed("GMAIL-SMTP-SETUP.md", &[]));
        assert!(is_doc_allowed(
            "GMAIL-FILTER-FORWARDING.zh-TW.md",
            &["zh-tw".to_string()]
        ));
        assert!(!is_doc_allowed(
            "RBAC.md",
            &["gmail".to_string()]
        ));

        assert!(language_bonus("RBAC.zh-TW.md", Some("zh-TW")) > language_bonus("RBAC.md", Some("zh-TW")));
        assert!(language_bonus("RBAC.md", Some("en")) > language_bonus("RBAC.zh-TW.md", Some("en")));
    }

    #[test]
    fn sanitize_path_component_normalizes_unsafe_chars() {
        assert_eq!(sanitize_path_component("A/B C?.txt"), "A_B_C_.txt");
        assert_eq!(sanitize_path_component("<>") , "__");
    }

    #[test]
    fn parse_mx_records_returns_highest_priority_host() {
        let output = "20 alt2.gmail-smtp-in.l.google.com.\n5 alt1.gmail-smtp-in.l.google.com.\n";
        assert_eq!(
            parse_mx_records(output),
            Some("alt1.gmail-smtp-in.l.google.com".to_string())
        );
    }

    #[test]
    fn parse_mx_records_ignores_invalid_rows() {
        let output = "invalid line\nabc mail.example.com.\n";
        assert_eq!(parse_mx_records(output), None);
    }

    #[test]
    fn parse_training_consent_answer_handles_yes_no() {
        assert_eq!(parse_training_consent_answer("Yes, I agree"), Some(true));
        assert_eq!(parse_training_consent_answer("我同意"), Some(true));
        assert_eq!(parse_training_consent_answer("No, I do not consent"), Some(false));
        assert_eq!(parse_training_consent_answer("不同意"), Some(false));
        assert_eq!(parse_training_consent_answer("maybe later"), None);
    }

    #[test]
    fn redact_training_text_masks_sensitive_patterns() {
        let raw = "Contact me at alice@example.com or +1 212-555-1234, token abcdefghijklmnopqrstuvwxyz123456";
        let redacted = redact_training_text(raw);
        assert!(!redacted.contains("alice@example.com"));
        assert!(!redacted.contains("212-555-1234"));
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_PHONE]"));
        assert!(redacted.contains("[REDACTED_TOKEN]"));
    }

    #[tokio::test]
    async fn capture_rule_from_chat_inserts_once_and_deduplicates() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE email_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                rule_text TEXT NOT NULL,
                source TEXT NOT NULL,
                is_enabled INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .expect("create email_rules");

        let user_id = "u1";
        let msg = "forward invoices to me and remind me";
        let first = capture_rule_from_chat(&pool, user_id, msg).await;
        let second = capture_rule_from_chat(&pool, user_id, msg).await;

        assert!(matches!(first, RuleCaptureOutcome::Created(_)));
        assert!(matches!(second, RuleCaptureOutcome::Duplicate(_)));

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn capture_rule_from_chat_skips_non_rule_messages() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("create in-memory db");

        sqlx::query(
            "CREATE TABLE email_rules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                rule_text TEXT NOT NULL,
                source TEXT NOT NULL,
                is_enabled INTEGER NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .expect("create email_rules");

        let outcome = capture_rule_from_chat(&pool, "u1", "hello there").await;
        assert!(matches!(outcome, RuleCaptureOutcome::None));

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(count, 0);
    }
}

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        "[redacted]".to_string()
    } else {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    }
}

async fn post_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> Json<Option<User>> {
    let token = payload.token.trim();
    tracing::info!(">>> [AUTH] Attempting to verify token: '{}'", mask_token(token));
    
    // Debug: check total tokens in DB
    let total_tokens: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE magic_token IS NOT NULL")
        .fetch_one(&state.pool).await.unwrap_or(0);
    tracing::info!(">>> [AUTH] System current has {} active magic tokens in DB.", total_tokens);

    let query_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE magic_token = ?")
        .bind(token)
        .fetch_optional(&state.pool).await;
        
    match query_result {
        Ok(Some(mut u)) => {
            tracing::info!(">>> [AUTH] SUCCESS: Token match found for user: {}", u.email);
            // Clear token after use
            let clear_res = sqlx::query("UPDATE users SET magic_token = NULL WHERE id = ?")
                .bind(&u.id)
                .execute(&state.pool).await;
            
            if let Err(e) = clear_res {
                tracing::error!(">>> [AUTH] FAILED to clear token for user {}: {:?}", u.email, e);
            }
                
            u.magic_token = None;
            u.role = role_for_email(&state, &u.email);
            Json(Some(u))
        },
        Ok(None) => {
            tracing::warn!(">>> [AUTH] FAILED: No user found with token '{}'.", mask_token(token));
            Json(None)
        },
        Err(e) => {
            tracing::error!(">>> [AUTH] DATABASE ERROR during verification: {:?}", e);
            Json(None)
        }
    }
}

#[derive(Deserialize)]
struct SettingsRequest {
    email: String,
    auto_reply: bool,
    dry_run: bool,
    email_format: String,
    training_data_consent: bool,
    timezone: Option<String>,
    preferred_language: Option<String>,
    display_name: Option<String>,
    assistant_name_zh: Option<String>,
    assistant_name_en: Option<String>,
    assistant_tone_zh: Option<String>,
    assistant_tone_en: Option<String>,
    pdf_passwords: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CreateRuleRequest {
    email: String,
    rule_text: String,
}

#[derive(Deserialize)]
struct UpdateRuleRequest {
    email: String,
    id: i64,
    rule_text: String,
}

#[derive(Deserialize)]
struct ToggleRuleRequest {
    email: String,
    id: i64,
    is_enabled: bool,
}

#[derive(Deserialize)]
struct DataDeletionRequestPayload {
    email: String,
}

#[derive(Deserialize)]
struct DataDeletionSummaryQuery {
    token: String,
}

#[derive(Deserialize)]
struct DataDeletionConfirmPayload {
    token: String,
    confirm: bool,
}

async fn post_settings(
    State(state): State<AppState>,
    Json(payload): Json<SettingsRequest>,
) -> Json<serde_json::Value> {
    let pdf_passwords_json = payload.pdf_passwords.as_ref().and_then(|v| serde_json::to_string(v).ok());
    let timezone = payload
        .timezone
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("UTC")
        .to_string();
    let preferred_language = payload
        .preferred_language
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("en")
        .to_string();
    
    let result = sqlx::query("UPDATE users SET auto_reply = ?, dry_run = ?, email_format = ?, training_data_consent = ?, \
                              training_consent_updated_at = CASE WHEN training_data_consent != ? THEN CURRENT_TIMESTAMP ELSE training_consent_updated_at END, \
                              timezone = ?, preferred_language = ?, display_name = ?, assistant_name_zh = ?, assistant_name_en = ?, assistant_tone_zh = ?, assistant_tone_en = ?, pdf_passwords = ? WHERE email = ?")
        .bind(payload.auto_reply)
        .bind(payload.dry_run)
        .bind(&payload.email_format)
        .bind(payload.training_data_consent)
        .bind(payload.training_data_consent)
        .bind(&timezone)
        .bind(&preferred_language)
        .bind(&payload.display_name)
        .bind(&payload.assistant_name_zh)
        .bind(&payload.assistant_name_en)
        .bind(&payload.assistant_tone_zh)
        .bind(&payload.assistant_tone_en)
        .bind(pdf_passwords_json)
        .bind(&payload.email)
        .execute(&state.pool).await;

        
    if result.is_ok() {
        let user_id = &payload.email; // Using email as user_id for stats for now
        let _ = OnboardingService::log_activity(&state.pool, user_id, "change_settings").await;
        if payload.auto_reply { let _ = OnboardingService::log_activity(&state.pool, user_id, "enable_auto_reply").await; }
        if payload.dry_run { let _ = OnboardingService::log_activity(&state.pool, user_id, "enable_dry_run").await; }
    }


        
    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update settings: {}", e);
            Json(serde_json::json!({ "status": "error" }))
        }
    }
}

async fn get_admin_errors(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let is_admin = query.email.as_deref()
        .map(|e| is_admin_or_developer(&state, e))
        .unwrap_or(false);

    if !is_admin {
        return Json(serde_json::json!({ "error": "Unauthorized" }));
    }

    let errors = sqlx::query_as::<_, MailError>(
        "SELECT m.id, m.level, m.error_type, m.message, m.context, m.user_id, u.email as user_email, CAST(m.occurred_at AS TEXT) as occurred_at \
         FROM mail_errors m \
         LEFT JOIN users u ON u.id = m.user_id \
         ORDER BY m.occurred_at DESC LIMIT 200"
    )
    .fetch_all(&state.pool).await.unwrap_or_default();

    Json(serde_json::json!({ "errors": errors }))
}

async fn get_user_errors(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "error": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "error": "User not found" }));
    };

    let errors = sqlx::query_as::<_, MailError>(
        "SELECT m.id, m.level, m.error_type, m.message, m.context, m.user_id, u.email as user_email, CAST(m.occurred_at AS TEXT) as occurred_at \
         FROM mail_errors m \
         LEFT JOIN users u ON u.id = m.user_id \
         WHERE m.user_id = ? \
         ORDER BY m.occurred_at DESC LIMIT 200"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "errors": errors }))
}

async fn get_rules(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rules = sqlx::query_as::<_, EmailRule>(
        "SELECT id, user_id, rule_text, source, is_enabled, CAST(created_at AS TEXT) as created_at, CAST(updated_at AS TEXT) as updated_at \
         FROM email_rules WHERE user_id = ? ORDER BY is_enabled DESC, updated_at DESC"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "status": "success", "rules": rules }))
}

async fn post_create_rule(
    State(state): State<AppState>,
    Json(payload): Json<CreateRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let result = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, source, is_enabled) VALUES (?, ?, 'manual', 1)"
    )
    .bind(&user_id)
    .bind(rule_text)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to create rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Create failed" }))
        }
    }
}

async fn post_update_rule(
    State(state): State<AppState>,
    Json(payload): Json<UpdateRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let result = sqlx::query(
        "UPDATE email_rules SET rule_text = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
    )
    .bind(rule_text)
    .bind(payload.id)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to update rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Update failed" }))
        }
    }
}

async fn post_toggle_rule(
    State(state): State<AppState>,
    Json(payload): Json<ToggleRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let result = sqlx::query(
        "UPDATE email_rules SET is_enabled = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
    )
    .bind(payload.is_enabled)
    .bind(payload.id)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Json(serde_json::json!({ "status": "success" })),
        Err(e) => {
            tracing::error!("Failed to toggle rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Toggle failed" }))
        }
    }
}

async fn send_data_deletion_confirmation_email(
    state: &AppState,
    user_email: &str,
    preferred_language: &str,
    snapshot: &DataDeletionSnapshot,
    confirm_url: &str,
) -> Result<(), String> {
    let smtp_ready = state.config.smtp_relay_host.as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);

    if !smtp_ready {
        return Err("SMTP relay is not configured (SMTP_RELAY_HOST missing or placeholder).".to_string());
    }

    let host = state.config.smtp_relay_host.clone().unwrap_or_default();
    let port = state.config.smtp_relay_port;
    let smtp_user = state.config.smtp_relay_user.clone().unwrap_or_default();
    let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();

    let (subject, text_body, html_body) = if preferred_language == "zh-TW" {
        (
            "AI Mail Butler - 請確認刪除您的資料",
            format!(
                "您已提出刪除 AI Mail Butler 個人資料的請求。\n\n目前資料統計：\n- 信件數：{}\n- 規則數：{}\n- Log 數：{}\n- 記憶資料數：{}\n- 活動統計列數：{}\n- 活動總事件次數：{}\n- 聊天紀錄數：{}\n- 檔案數：{}\n- 檔案總大小：{} bytes\n\n重要提醒：資料庫快取頁或已覆寫頁面可能無法復原。系統目前僅能協助刪除，無法協助取回。\n\n步驟 1：請先開啟下列確認連結再次檢視報表：\n{}\n\n開啟後仍需在頁面進行第二次最終確認才會刪除。",
                snapshot.email_count,
                snapshot.rule_count,
                snapshot.log_count,
                snapshot.memory_count,
                snapshot.activity_row_count,
                snapshot.activity_event_total,
                snapshot.chat_log_count,
                snapshot.file_count,
                snapshot.total_file_bytes,
                confirm_url,
            ),
            format!(
                "<h2>資料刪除確認</h2><p>您已提出刪除所有資料的請求。</p><ul><li>信件數：{}</li><li>規則數：{}</li><li>Log 數：{}</li><li>記憶資料數：{}</li><li>活動統計列數：{}</li><li>活動總事件次數：{}</li><li>聊天紀錄數：{}</li><li>檔案數：{}</li><li>檔案總大小：{} bytes</li></ul><p><strong>重要提醒：</strong>資料庫快取頁或已覆寫頁面可能無法復原。系統目前僅能協助刪除，無法協助取回。</p><p><a href=\"{}\">開啟確認報表</a></p><p>開啟後仍需進行第二次最終確認才會刪除。</p>",
                snapshot.email_count,
                snapshot.rule_count,
                snapshot.log_count,
                snapshot.memory_count,
                snapshot.activity_row_count,
                snapshot.activity_event_total,
                snapshot.chat_log_count,
                snapshot.file_count,
                snapshot.total_file_bytes,
                confirm_url,
            ),
        )
    } else {
        (
            "AI Mail Butler - Confirm Your Data Deletion Request",
            format!(
                "You requested to delete all your data from AI Mail Butler.\n\nCurrent data snapshot:\n- Emails: {}\n- Rules: {}\n- Logs: {}\n- Memories: {}\n- Activity rows: {}\n- Activity event total: {}\n- Chat logs: {}\n- Files: {}\n- Total file size: {} bytes\n\nImportant: Cached or already-overwritten database pages may not be recoverable after deletion. The system can only assist with deletion and cannot restore deleted data.\n\nStep 1: Open this confirmation link to review your report again:\n{}\n\nAfter opening, you must still do a second final confirmation on the report page.",
                snapshot.email_count,
                snapshot.rule_count,
                snapshot.log_count,
                snapshot.memory_count,
                snapshot.activity_row_count,
                snapshot.activity_event_total,
                snapshot.chat_log_count,
                snapshot.file_count,
                snapshot.total_file_bytes,
                confirm_url,
            ),
            format!(
                "<h2>Data Deletion Confirmation</h2><p>You requested deletion of all your data.</p><ul><li>Emails: {}</li><li>Rules: {}</li><li>Logs: {}</li><li>Memories: {}</li><li>Activity rows: {}</li><li>Activity event total: {}</li><li>Chat logs: {}</li><li>Files: {}</li><li>Total file size: {} bytes</li></ul><p><strong>Important:</strong> Cached or overwritten database pages may not be recoverable after deletion. The system can only assist with deletion and cannot restore deleted data.</p><p><a href=\"{}\">Open confirmation report</a></p><p>After opening the report, you still need a second final confirmation to delete.</p>",
                snapshot.email_count,
                snapshot.rule_count,
                snapshot.log_count,
                snapshot.memory_count,
                snapshot.activity_row_count,
                snapshot.activity_event_total,
                snapshot.chat_log_count,
                snapshot.file_count,
                snapshot.total_file_bytes,
                confirm_url,
            ),
        )
    };

    let assistant_name = if preferred_language == "zh-TW" {
        "AI 郵件助理"
    } else {
        "AI Mail Butler"
    };
    let assistant_mailbox = format!("{} <{}>", assistant_name, state.config.assistant_email);

    let message = MessageBuilder::new()
        .from(assistant_mailbox.clone())
        .reply_to(assistant_mailbox)
        .to(user_email.to_string())
        .subject(subject)
        .text_body(text_body)
        .html_body(html_body);

    let is_implicit = port == 465;
    let mut builder = SmtpClientBuilder::new(host.as_str(), port).implicit_tls(is_implicit);
    if !smtp_user.is_empty() {
        builder = builder.credentials((smtp_user.as_str(), pass.as_str()));
    }

    match builder.connect().await {
        Ok(mut client) => {
            match client.send(message).await {
                Ok(_) => Ok(()),
                Err(e) => Err(format!(
                    "SMTP send failed to {} via {}:{} ({})",
                    user_email,
                    host,
                    port,
                    e
                )),
            }
        }
        Err(e) => {
            Err(format!(
                "SMTP connect failed to {}:{} while sending GDPR confirmation to {} ({})",
                host,
                port,
                user_email,
                e
            ))
        }
    }
}

async fn post_request_data_deletion(
    State(state): State<AppState>,
    Json(payload): Json<DataDeletionRequestPayload>,
) -> Json<serde_json::Value> {
    let email = payload.email.trim().to_ascii_lowercase();
    let user_row: Option<(String, String)> = sqlx::query_as("SELECT id, preferred_language FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    let Some((user_id, preferred_language)) = user_row else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let snapshot = build_user_data_snapshot(&state.pool, &user_id, &email).await;
    let snapshot_json = serde_json::to_string(&snapshot).unwrap_or_else(|_| "{}".to_string());
    let token = uuid::Uuid::new_v4().to_string();
    let req_id = uuid::Uuid::new_v4().to_string();

    let _ = sqlx::query(
        "INSERT INTO data_deletion_requests (id, user_id, token, status, snapshot_json) VALUES (?, ?, ?, 'requested', ?)"
    )
    .bind(&req_id)
    .bind(&user_id)
    .bind(&token)
    .bind(&snapshot_json)
    .execute(&state.pool)
    .await;

    let base_url = std::env::var("PUBLIC_URL")
        .unwrap_or_else(|_| format!("http://localhost:{}", state.config.server_port));
    let confirm_url = format!("{}/gdpr-delete?token={}", base_url, token);

    let send_result = send_data_deletion_confirmation_email(&state, &email, &preferred_language, &snapshot, &confirm_url).await;
    let delivered = send_result.is_ok();
    if let Err(reason) = send_result {
        let msg = format!("Failed to deliver data deletion confirmation email to {}: {}", email, reason);
        tracing::error!("{}", msg);
        let context = format!("confirm_url={}; reason={}", confirm_url, reason);
        log_mail_event(&state.pool, "ERROR", "gdpr_email_send", &msg, Some(&context), Some(&user_id)).await;
    }

    Json(serde_json::json!({
        "status": "success",
        "delivered": delivered,
        "message": "Deletion request recorded. Please check your email for the confirmation link.",
    }))
}

async fn get_data_deletion_summary(
    State(state): State<AppState>,
    Query(query): Query<DataDeletionSummaryQuery>,
) -> Json<serde_json::Value> {
    let row = sqlx::query_as::<_, DataDeletionRequestRow>(
        "SELECT r.id, r.user_id, u.email as user_email, r.status, r.snapshot_json \
         FROM data_deletion_requests r \
         JOIN users u ON u.id = r.user_id \
         WHERE r.token = ?"
    )
    .bind(query.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(serde_json::json!({ "status": "error", "message": "Invalid or expired token" }));
    };

    if row.status == "finalized" {
        return Json(serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }));
    }

    if row.status == "requested" {
        let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'email_confirmed', email_confirmed_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&row.id)
            .execute(&state.pool)
            .await;
    }

    let snapshot: DataDeletionSnapshot = serde_json::from_str(&row.snapshot_json).unwrap_or_default();

    Json(serde_json::json!({
        "status": "ready",
        "email": row.user_email,
        "snapshot": snapshot,
        "warning": "Cached/overwritten database pages may not be recoverable. The system can only assist deletion and cannot restore deleted data.",
        "require_second_confirmation": true
    }))
}

async fn post_confirm_data_deletion(
    State(state): State<AppState>,
    Json(payload): Json<DataDeletionConfirmPayload>,
) -> Json<serde_json::Value> {
    if !payload.confirm {
        return Json(serde_json::json!({ "status": "error", "message": "Final confirmation is required" }));
    }

    let row = sqlx::query_as::<_, DataDeletionRequestRow>(
        "SELECT r.id, r.user_id, u.email as user_email, r.status, r.snapshot_json \
         FROM data_deletion_requests r \
         JOIN users u ON u.id = r.user_id \
         WHERE r.token = ?"
    )
    .bind(payload.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(serde_json::json!({ "status": "error", "message": "Invalid or expired token" }));
    };

    if row.status == "finalized" {
        return Json(serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }));
    }

    let sender_dir = format!("data/mail_spool/{}", sanitize_path_component(&row.user_email.to_ascii_lowercase()));

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return Json(serde_json::json!({ "status": "error", "message": format!("Cannot start deletion transaction: {}", e) }));
        }
    };

    let _ = sqlx::query("DELETE FROM emails WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM email_rules WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM user_memories WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM user_activity_stats WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM mail_errors WHERE user_id = ?").bind(&row.user_id).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM chat_logs WHERE user_email = ?").bind(&row.user_email).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM chat_transcripts WHERE user_email = ?").bind(&row.user_email).execute(&mut *tx).await;
    let _ = sqlx::query("DELETE FROM chat_feedback WHERE user_email = ?").bind(&row.user_email).execute(&mut *tx).await;
    let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'finalized', finalized_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&row.id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = ?").bind(&row.user_id).execute(&mut *tx).await;

    if let Err(e) = tx.commit().await {
        return Json(serde_json::json!({ "status": "error", "message": format!("Deletion failed: {}", e) }));
    }

    let _ = fs::remove_dir_all(&sender_dir).await;

    Json(serde_json::json!({
        "status": "success",
        "message": "All removable user data has been deleted. Cached/overwritten database pages are not recoverable.",
    }))
}

pub async fn start_server(port: u16, state: AppState) -> Result<()> {
    let api_router = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/about", get(get_about))
        .route("/auth/magic-link", post(post_magic_link))
        .route("/auth/verify", post(post_verify))
        .route("/me", get(get_me))
        .route("/settings", post(post_settings))
        .route("/data-deletion/request", post(post_request_data_deletion))
        .route("/data-deletion/summary", get(get_data_deletion_summary))
        .route("/data-deletion/confirm", post(post_confirm_data_deletion))
        .route("/rules", get(get_rules))
        .route("/rules/create", post(post_create_rule))
        .route("/rules/update", post(post_update_rule))
        .route("/rules/toggle", post(post_toggle_rule))
        .route("/dashboard", get(get_dashboard))
        .route("/errors", get(get_user_errors))
        .route("/admin/errors", get(get_admin_errors))
        .route("/training/export", get(get_training_export))
        .route("/feedback", get(get_feedback))
        .route("/feedback/mark-read", post(post_mark_feedback_read))
        .route("/feedback/reply", post(post_reply_feedback))
        .route("/chat", post(post_chat))
        .route("/chat/feedback", post(post_chat_feedback));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html"))
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
