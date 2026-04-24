use anyhow::Result;
use axum::{
    body::Body,
    extract::{Query, State},
    http::{Method, Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::fs;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::ai::AiClient;
use crate::models::{
    ConsentAuditTrail, DataRetentionPolicy, DsarRequest, EmailRecord, User, UserAgeVerification,
    UserPrivacySettings,
};
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
    rule_label: String,
    source: String,
    is_enabled: bool,
    matched_count: i64,
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

#[derive(sqlx::FromRow, Serialize)]
struct FinanceRecordRow {
    id: String,
    email_id: String,
    subject: Option<String>,
    reason: String,
    category: String,
    direction: String,
    amount: f64,
    currency: String,
    month_key: String,
    month_total_after: f64,
    finance_type: Option<String>,
    due_date: Option<String>,
    statement_amount: Option<f64>,
    issuing_bank: Option<String>,
    card_last4: Option<String>,
    transaction_month_key: Option<String>,
    created_at: String,
}

#[derive(sqlx::FromRow, Serialize)]
struct MonthlyFinanceSummaryRow {
    month_key: String,
    category: String,
    total_amount: f64,
    updated_at: String,
}

fn parse_training_consent_answer(message: &str) -> Option<bool> {
    let m = message.trim().to_lowercase();
    let yes_markers = [
        "yes", "agree", "i agree", "consent", "allow", "ok", "同意", "願意", "可以", "好", "是",
    ];
    let no_markers = [
        "no",
        "disagree",
        "do not",
        "don't",
        "deny",
        "不同意",
        "不願意",
        "不要",
        "否",
        "不行",
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

    let email_re = EMAIL_RE.get_or_init(|| {
        Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email regex")
    });
    let us_phone_re = US_PHONE_RE.get_or_init(|| {
        Regex::new(r"(?:\+?1[-.\s]?)?(?:\(?\d{3}\)?[-.\s]?)\d{3}[-.\s]?\d{4}")
            .expect("us phone regex")
    });
    let tw_phone_re = TW_PHONE_RE.get_or_init(|| {
        Regex::new(r"(?:\+886[-.\s]?)?0?9\d{2}[-.\s]?\d{3}[-.\s]?\d{3}").expect("tw phone regex")
    });
    let long_token_re =
        LONG_TOKEN_RE.get_or_init(|| Regex::new(r"\b[A-Za-z0-9_-]{24,}\b").expect("token regex"));

    let out = email_re.replace_all(input, "[REDACTED_EMAIL]").to_string();
    let out = us_phone_re
        .replace_all(&out, "[REDACTED_PHONE]")
        .to_string();
    let out = tw_phone_re
        .replace_all(&out, "[REDACTED_PHONE]")
        .to_string();
    long_token_re
        .replace_all(&out, "[REDACTED_TOKEN]")
        .to_string()
}

fn generate_rule_label(rule_text: &str) -> String {
    fn is_cjk(c: char) -> bool {
        matches!(
            c as u32,
            0x4E00..=0x9FFF    // CJK Unified Ideographs
                | 0x3400..=0x4DBF // CJK Extension A
                | 0xF900..=0xFAFF // CJK Compatibility Ideographs
        )
    }

    let trimmed = rule_text.trim();
    if trimmed.is_empty() {
        return "RULE".to_string();
    }

    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in trimmed.chars() {
        if ch.is_alphanumeric() || is_cjk(ch) {
            current.push(ch);
        } else if !current.is_empty() {
            tokens.push(current.clone());
            current.clear();
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    let zh_stop = [
        "如果", "請", "先", "再", "和", "或", "的", "與", "就", "並", "把", "將", "為", "是",
    ];
    let en_stop = [
        "the", "a", "an", "and", "or", "if", "then", "for", "to", "of", "on", "in", "with",
    ];

    let mut picked: Vec<String> = Vec::new();
    for token in tokens {
        let lower = token.to_lowercase();
        if en_stop.contains(&lower.as_str()) || zh_stop.contains(&token.as_str()) {
            continue;
        }

        let clipped = if token.chars().any(is_cjk) {
            token.chars().take(4).collect::<String>()
        } else {
            lower.chars().take(12).collect::<String>()
        };

        if !clipped.is_empty() {
            picked.push(clipped);
        }
        if picked.len() >= 3 {
            break;
        }
    }

    let mut core = if picked.is_empty() {
        trimmed
            .chars()
            .filter(|c| c.is_alphanumeric() || is_cjk(*c))
            .take(8)
            .collect::<String>()
    } else if picked.iter().any(|t| t.chars().any(is_cjk)) {
        picked.join("")
    } else {
        picked.join("-")
    };

    if core.is_empty() {
        core = "RULE".to_string();
    }

    let core: String = core.chars().take(18).collect();
    format!("RULE-{}", core)
}

fn ai_rule_label_enabled() -> bool {
    match std::env::var("RULE_LABEL_USE_AI") {
        Ok(v) => {
            let normalized = v.trim().to_ascii_lowercase();
            !matches!(normalized.as_str(), "0" | "false" | "no" | "off")
        }
        Err(_) => true,
    }
}

fn sanitize_ai_rule_label(raw: &str) -> Option<String> {
    fn is_cjk(c: char) -> bool {
        matches!(
            c as u32,
            0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF
        )
    }

    let mut first = raw.lines().next().unwrap_or_default().trim().to_string();
    if first.is_empty() {
        return None;
    }

    if let Some((_, right)) = first.split_once(':') {
        if first.to_ascii_lowercase().starts_with("name")
            || first.to_ascii_lowercase().starts_with("label")
        {
            first = right.trim().to_string();
        }
    }

    first = first
        .trim_matches('"')
        .trim_matches('`')
        .trim_matches('\'')
        .to_string();

    if first.to_ascii_lowercase().starts_with("rule-") {
        first = first[5..].trim().to_string();
    }

    let has_cjk = first.chars().any(is_cjk);
    let mut normalized = String::new();
    let mut prev_dash = false;
    for ch in first.chars() {
        if ch.is_alphanumeric() || is_cjk(ch) || ch == '_' || ch == '-' {
            normalized.push(ch);
            prev_dash = false;
        } else if ch.is_whitespace() && !has_cjk && !prev_dash && !normalized.is_empty() {
            normalized.push('-');
            prev_dash = true;
        }
    }

    let normalized = normalized.trim_matches('-').trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    let core: String = normalized.chars().take(18).collect();
    if core.is_empty() {
        None
    } else {
        Some(format!("RULE-{}", core))
    }
}

async fn generate_rule_label_with_ai(
    ai_client: Option<&AiClient>,
    rule_text: &str,
    rule_label_mode: Option<&str>,
) -> String {
    let fallback = generate_rule_label(rule_text);
    let use_ai =
        matches!(rule_label_mode.unwrap_or("ai_first"), "ai_first") && ai_rule_label_enabled();
    if !use_ai {
        return fallback;
    }

    let Some(client) = ai_client else {
        return fallback;
    };

    let system_prompt = "You create very short rule names for email automation. Output only one concise label core without prefix. Rules: 1) 2-8 Chinese chars OR 1-3 English words. 2) No punctuation except hyphen/underscore. 3) Avoid generic words like rule, email, message. 4) Keep it specific to intent.";
    let user_prompt = format!(
        "Rule description:\n{}\n\nReturn only the label core (without RULE- prefix).",
        rule_text
    );

    match client.chat(system_prompt, &user_prompt).await {
        Ok(res) => sanitize_ai_rule_label(&res.content).unwrap_or(fallback),
        Err(e) => {
            tracing::warn!(
                "AI rule label generation failed, fallback to algorithm: {}",
                e
            );
            fallback
        }
    }
}

fn pending_rule_delete_map() -> &'static RwLock<HashMap<String, i64>> {
    static MAP: OnceLock<RwLock<HashMap<String, i64>>> = OnceLock::new();
    MAP.get_or_init(|| RwLock::new(HashMap::new()))
}

fn extract_first_rule_id(message: &str) -> Option<i64> {
    static RULE_ID_RE: OnceLock<Regex> = OnceLock::new();
    let re = RULE_ID_RE.get_or_init(|| Regex::new(r"\b(\d{1,9})\b").expect("rule id regex"));
    let cap = re.captures(message)?;
    cap.get(1)?.as_str().parse::<i64>().ok()
}

fn extract_rule_label_token(message: &str) -> Option<String> {
    static RULE_LABEL_RE: OnceLock<Regex> = OnceLock::new();
    let re = RULE_LABEL_RE
        .get_or_init(|| Regex::new(r"(?iu)\b(rule-[\p{L}\p{N}_-]+)\b").expect("rule label regex"));
    let cap = re.captures(message)?;
    Some(cap.get(1)?.as_str().to_string())
}

fn extract_text_after_delimiter(message: &str) -> Option<String> {
    for sep in [':', '：'] {
        if let Some((_, right)) = message.split_once(sep) {
            let t = right.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn is_confirm_delete_phrase(lower: &str) -> bool {
    [
        "確認",
        "確認刪除",
        "是",
        "好",
        "同意",
        "刪除吧",
        "確定刪除",
        "confirm",
        "yes",
        "y",
        "delete it",
        "proceed",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn is_cancel_delete_phrase(lower: &str) -> bool {
    ["取消", "不要", "否", "no", "cancel", "stop"]
        .iter()
        .any(|k| lower.contains(k))
}

fn is_rule_count_query(lower: &str) -> bool {
    [
        "幾條規則",
        "多少規則",
        "規則數量",
        "有幾個規則",
        "現在規則",
        "how many rule",
        "rule count",
        "number of rules",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn is_rule_list_query(lower: &str) -> bool {
    [
        "列出規則",
        "顯示規則",
        "查看規則",
        "list rules",
        "show rules",
    ]
    .iter()
    .any(|k| lower.contains(k))
}

fn is_rule_edit_intent(lower: &str) -> bool {
    ["編輯規則", "修改規則", "update rule", "edit rule"]
        .iter()
        .any(|k| lower.contains(k))
}

fn is_rule_disable_intent(lower: &str) -> bool {
    ["停用規則", "關閉規則", "disable rule", "turn off rule"]
        .iter()
        .any(|k| lower.contains(k))
}

fn is_rule_delete_intent(lower: &str) -> bool {
    ["刪除規則", "delete rule", "remove rule"]
        .iter()
        .any(|k| lower.contains(k))
}

async fn handle_rule_chat_command(
    pool: &SqlitePool,
    ai_client: &AiClient,
    user: &User,
    message: &str,
) -> Option<String> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_lowercase();
    let zh = user.preferred_language == "zh-TW";

    {
        let map = pending_rule_delete_map();
        let pending = map.read().await.get(&user.id).cloned();
        if let Some(rule_id) = pending {
            if is_confirm_delete_phrase(&lower) {
                let result = sqlx::query("DELETE FROM email_rules WHERE id = ? AND user_id = ?")
                    .bind(rule_id)
                    .bind(&user.id)
                    .execute(pool)
                    .await;

                map.write().await.remove(&user.id);

                return Some(match result {
                    Ok(r) if r.rows_affected() > 0 => {
                        if zh {
                            format!("已刪除規則 #{}。", rule_id)
                        } else {
                            format!("Deleted rule #{}.", rule_id)
                        }
                    }
                    _ => {
                        if zh {
                            "刪除失敗，可能規則已不存在。".to_string()
                        } else {
                            "Delete failed, rule may no longer exist.".to_string()
                        }
                    }
                });
            }

            if is_cancel_delete_phrase(&lower) {
                map.write().await.remove(&user.id);
                return Some(if zh {
                    "已取消刪除。".to_string()
                } else {
                    "Deletion canceled.".to_string()
                });
            }
        }
    }

    if is_rule_count_query(&lower) {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules WHERE user_id = ?")
            .bind(&user.id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        return Some(if zh {
            format!("你目前有 {} 條規則。", count)
        } else {
            format!("You currently have {} rules.", count)
        });
    }

    if is_rule_list_query(&lower) {
        let rules: Vec<(i64, String, String, i64, bool)> = sqlx::query_as(
            "SELECT id, rule_label, rule_text, matched_count, is_enabled FROM email_rules WHERE user_id = ? ORDER BY is_enabled DESC, updated_at DESC LIMIT 20"
        )
        .bind(&user.id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        if rules.is_empty() {
            return Some(if zh {
                "你目前還沒有規則。".to_string()
            } else {
                "You don't have any rules yet.".to_string()
            });
        }

        let lines = rules
            .into_iter()
            .map(|(id, label, text, matched, enabled)| {
                let state = if enabled { "enabled" } else { "disabled" };
                format!(
                    "#{} [{}] ({}, matched {}) {}",
                    id, label, state, matched, text
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        return Some(if zh {
            format!("目前規則如下：\n{}", lines)
        } else {
            format!("Current rules:\n{}", lines)
        });
    }

    if is_rule_edit_intent(&lower) {
        let rule_id = extract_first_rule_id(trimmed);
        let new_text = extract_text_after_delimiter(trimmed);
        let (Some(rule_id), Some(new_text)) = (rule_id, new_text) else {
            return Some(if zh {
                "請用這種格式：編輯規則 12：新的規則內容".to_string()
            } else {
                "Use format: edit rule 12: new rule text".to_string()
            });
        };

        let generated_label =
            generate_rule_label_with_ai(Some(ai_client), &new_text, Some(&user.rule_label_mode))
                .await;
        let result = sqlx::query(
            "UPDATE email_rules SET rule_text = ?, rule_label = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
        )
        .bind(&new_text)
        .bind(generated_label)
        .bind(rule_id)
        .bind(&user.id)
        .execute(pool)
        .await;

        return Some(match result {
            Ok(r) if r.rows_affected() > 0 => {
                if zh {
                    format!("已更新規則 #{}。", rule_id)
                } else {
                    format!("Updated rule #{}.", rule_id)
                }
            }
            _ => {
                if zh {
                    "找不到可更新的規則編號。".to_string()
                } else {
                    "Could not find that rule id to update.".to_string()
                }
            }
        });
    }

    if is_rule_disable_intent(&lower) {
        let rule_id = extract_first_rule_id(trimmed);
        let Some(rule_id) = rule_id else {
            return Some(if zh {
                "請指定要停用的規則編號，例如：停用規則 12".to_string()
            } else {
                "Please provide the rule id, e.g. disable rule 12".to_string()
            });
        };

        let result = sqlx::query(
            "UPDATE email_rules SET is_enabled = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
        )
        .bind(rule_id)
        .bind(&user.id)
        .execute(pool)
        .await;

        return Some(match result {
            Ok(r) if r.rows_affected() > 0 => {
                if zh {
                    format!("已停用規則 #{}。", rule_id)
                } else {
                    format!("Disabled rule #{}.", rule_id)
                }
            }
            _ => {
                if zh {
                    "找不到可停用的規則編號。".to_string()
                } else {
                    "Could not find that rule id to disable.".to_string()
                }
            }
        });
    }

    if is_rule_delete_intent(&lower) {
        let mut rule_id = extract_first_rule_id(trimmed);
        if rule_id.is_none() {
            if let Some(label) = extract_rule_label_token(trimmed) {
                let found: Option<i64> = sqlx::query_scalar("SELECT id FROM email_rules WHERE user_id = ? AND lower(rule_label) = lower(?) LIMIT 1")
                    .bind(&user.id)
                    .bind(label)
                    .fetch_optional(pool)
                    .await
                    .ok()
                    .flatten();
                rule_id = found;
            }
        }

        let Some(rule_id) = rule_id else {
            return Some(if zh {
                "請指定要刪除的規則編號或標籤，例如：刪除規則 12，或刪除規則 RULE-INVOICE"
                    .to_string()
            } else {
                "Please specify rule id or label, e.g. delete rule 12 or delete rule RULE-INVOICE"
                    .to_string()
            });
        };

        let exists: Option<(String,)> = sqlx::query_as(
            "SELECT rule_text FROM email_rules WHERE id = ? AND user_id = ? LIMIT 1",
        )
        .bind(rule_id)
        .bind(&user.id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();

        let Some((rule_text,)) = exists else {
            return Some(if zh {
                "找不到該規則。".to_string()
            } else {
                "Rule not found.".to_string()
            });
        };

        pending_rule_delete_map()
            .write()
            .await
            .insert(user.id.clone(), rule_id);
        return Some(if zh {
            format!(
                "你即將刪除規則 #{}：{}。\n請回覆「確認刪除」以執行，或回覆「取消」放棄。",
                rule_id, rule_text
            )
        } else {
            format!("You are about to delete rule #{}: {}.\nReply 'confirm' to proceed or 'cancel' to abort.", rule_id, rule_text)
        });
    }

    None
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

async fn build_docs_context(
    query: &str,
    preferred_language: Option<&str>,
    docs_whitelist: &[String],
) -> Option<String> {
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

        let score: usize = terms
            .iter()
            .map(|t| entry.lower_content.matches(t).count())
            .sum();
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
        "轉寄",
        "請幫我",
        "遇到",
        "收到",
        "帳單",
        "通知",
        "發票",
        "提醒",
        "回覆",
        "處理",
        "規則",
        "新增規則",
        "建立規則",
        "forward",
        "when",
        "invoice",
        "bill",
        "receipt",
        "notify",
        "remind",
        "reply",
        "urgent",
        "important",
        "rule",
        "add rule",
        "create rule",
        "set a rule",
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
        "新增規則",
        "建立規則",
        "設定規則",
        "幫我新增規則",
        "幫我建立規則",
        "add rule",
        "create rule",
        "set a rule",
        "new rule",
        "rule",
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
                Some(format!(
                    "[Rule Created] I created a new rule from your request: {}",
                    rule
                ))
            }
        }
        RuleCaptureOutcome::Duplicate(rule) => {
            if preferred_language == "zh-TW" {
                Some(format!(
                    "[Rule Exists] 這條規則已存在，已略過重複新增：{}",
                    rule
                ))
            } else {
                Some(format!(
                    "[Rule Exists] This rule already exists, so I skipped creating a duplicate: {}",
                    rule
                ))
            }
        }
        RuleCaptureOutcome::None => None,
    }
}

async fn capture_rule_from_chat(
    pool: &SqlitePool,
    ai_client: Option<&AiClient>,
    user_id: &str,
    message: &str,
    rule_label_mode: Option<&str>,
) -> RuleCaptureOutcome {
    let Some(cleaned) = extract_rule_from_message(message) else {
        return RuleCaptureOutcome::None;
    };

    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM email_rules WHERE user_id = ? AND lower(rule_text) = lower(?) LIMIT 1",
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

    let generated_label = generate_rule_label_with_ai(ai_client, &cleaned, rule_label_mode).await;
    let insert_result = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, rule_label, source, is_enabled) VALUES (?, ?, ?, 'chat', 1)"
    )
    .bind(user_id)
    .bind(&cleaned)
    .bind(generated_label)
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

async fn collect_dir_stats(base_dir: &str, extra_base_dir: Option<&str>) -> (i64, i64) {
    let mut total_files = 0_i64;
    let mut total_bytes = 0_i64;
    let mut stack = vec![PathBuf::from(base_dir)];
    if let Some(extra) = extra_base_dir {
        stack.push(PathBuf::from(extra));
    }

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

async fn build_user_data_snapshot(
    pool: &SqlitePool,
    user_id: &str,
    user_email: &str,
    sender_dir: &str,
    config: &crate::config::Config,
) -> DataDeletionSnapshot {
    // In readonly mode also count files from the base sender directory.
    let extra_base: Option<String> = if config.readonly_mode_enabled {
        let overlay_root = config.overlay_dir.as_deref().unwrap_or("data/overlay");
        config.readonly_base.as_deref().and_then(|base| {
            let runtime_pb = std::path::Path::new(sender_dir);
            let overlay_pb = std::path::Path::new(overlay_root);
            runtime_pb.strip_prefix(overlay_pb).ok().map(|rel| {
                std::path::Path::new(base)
                    .join(rel)
                    .to_string_lossy()
                    .into_owned()
            })
        })
    } else {
        None
    };
    let (file_count, total_file_bytes) = collect_dir_stats(sender_dir, extra_base.as_deref()).await;

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
    let memory_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_memories WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    let activity_row_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM user_activity_stats WHERE user_id = ?")
            .bind(user_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    let activity_event_total: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(count), 0) FROM user_activity_stats WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);
    let chat_log_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs WHERE user_email = ?")
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

fn resolve_runtime_mail_path(state: &AppState, logical_path: &str) -> PathBuf {
    if !state.config.readonly_mode_enabled {
        return PathBuf::from(logical_path);
    }

    let overlay_root = state
        .config
        .overlay_dir
        .as_deref()
        .unwrap_or("data/overlay");
    let overlay_root_path = PathBuf::from(overlay_root);
    let logical = PathBuf::from(logical_path);
    if logical.starts_with(&overlay_root_path) {
        return logical;
    }

    if logical.is_absolute() {
        overlay_root_path.join(logical.strip_prefix("/").unwrap_or(&logical))
    } else {
        overlay_root_path.join(logical)
    }
}

async fn readonly_write_guard(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    if !state.config.readonly_mode_enabled {
        return next.run(req).await;
    }

    let method = req.method();
    let path = req.uri().path();
    let read_only_method = matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS);
    if read_only_method {
        return next.run(req).await;
    }

    // Allow authentication bootstrap while all other writes are blocked.
    if path == "/api/auth/magic-link" || path == "/api/auth/verify" {
        return next.run(req).await;
    }

    let payload = serde_json::json!({
        "status": "error",
        "message": "Read-only overlay mode is enabled. Write operations are blocked.",
    });
    (StatusCode::SERVICE_UNAVAILABLE, Json(payload)).into_response()
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

#[derive(Deserialize)]
struct ManualProcessRequest {
    email: String,
    email_ids: Vec<String>,
    force_reextract: Option<bool>,
}

#[derive(Deserialize)]
struct RetryMailErrorRequest {
    email: String,
    error_id: i64,
}

fn resolve_mail_error_source_path(state: &AppState, context: &str) -> Option<PathBuf> {
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut candidates = vec![PathBuf::from(trimmed)];
    if let Some(file_name) = PathBuf::from(trimmed).file_name().and_then(|s| s.to_str()) {
        candidates.push(resolve_runtime_mail_path(
            state,
            &format!("data/mail_spool/processed/{}", file_name),
        ));
        candidates.push(resolve_runtime_mail_path(
            state,
            &format!("data/mail_spool/{}", file_name),
        ));
    }

    candidates.into_iter().find(|p| p.is_file())
}

async fn get_me(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<Option<User>> {
    if let Some(email) = query.email {
        if email.is_empty() {
            return Json(None);
        }

        // Use UPSERT logic or separate check to handle concurrency/re-registrations
        let role = role_for_email(&state, &email);
        if !state.config.readonly_mode_enabled {
            let new_id = uuid::Uuid::new_v4().to_string();
            let _ = sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
                .bind(&new_id)
                .bind(&email)
                .execute(&state.pool)
                .await;
        }

        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or(None);

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
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let received_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails")
        .fetch_one(pool)
        .await
        .unwrap_or(0);
    let replied_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE status = 'replied'")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
    let ai_replies_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM chat_logs")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

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
                .fetch_optional(pool)
                .await
                .unwrap_or(None);

            if let Some((uid,)) = user_id {
                let personal_emails = sqlx::query_as::<_, EmailRecord>("SELECT id, subject, preview, status, matched_rule_label, CAST(received_at AS TEXT) as received_at FROM emails WHERE user_id = ? ORDER BY received_at DESC")
                    .bind(&uid)
                    .fetch_all(pool).await.unwrap_or(vec![]);

                let p_received: i64 =
                    sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE user_id = ?")
                        .bind(&uid)
                        .fetch_one(pool)
                        .await
                        .unwrap_or(0);
                let p_replied: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM emails WHERE user_id = ? AND status = 'replied'",
                )
                .bind(&uid)
                .fetch_one(pool)
                .await
                .unwrap_or(0);

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
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

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

            if let Some(command_reply) =
                handle_rule_chat_command(pool, &state.ai_client, &user, &message).await
            {
                let _ = sqlx::query(
                    "INSERT INTO chat_transcripts (user_id, user_email, user_message, ai_reply) VALUES (?, ?, ?, ?)"
                )
                .bind(&user.id)
                .bind(&user.email)
                .bind(&message)
                .bind(&command_reply)
                .execute(pool)
                .await;

                sqlx::query("INSERT INTO chat_logs (user_email) VALUES (?)")
                    .bind(&user.email)
                    .execute(pool)
                    .await
                    .ok();

                return Json(serde_json::json!({
                    "reply": command_reply,
                    "total_tokens": 0,
                    "duration_ms": 0,
                    "finish_reason": "rule_command",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }));
            }

            // Update preferences
            let new_pref =
                match OnboardingService::extract_preferences(&state.ai_client, &user, &message)
                    .await
                {
                    Ok(p) => p,
                    Err(_) => user.preferences.clone().unwrap_or_default(),
                };
            sqlx::query("UPDATE users SET preferences = ?, is_onboarded = true WHERE id = ?")
                .bind(&new_pref)
                .bind(&user.id)
                .execute(pool)
                .await
                .ok();
            user.preferences = Some(new_pref);

            let docs_context = build_docs_context(
                &message,
                Some(user.preferred_language.as_str()),
                &state.config.docs_whitelist,
            )
            .await;

            let memory = OnboardingService::get_memory(pool, &user.id).await;
            let mut res = match OnboardingService::generate_reply(
                &state.ai_client,
                &user,
                &message,
                &memory,
                &state.config.assistant_email,
                None,
                docs_context.clone(),
            )
            .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("Failed to generate reply: {}", e);
                    crate::ai::ChatResult {
                        content: "Sorry, I am having trouble connecting to my AI brain."
                            .to_string(),
                        total_tokens: 0,
                        duration_ms: 0,
                        finish_reason: None,
                    }
                }
            };

            // Detect repetitive questions (simple keyword for now)
            let lower_msg = message.to_lowercase();
            if lower_msg.contains("forward")
                || lower_msg.contains("email")
                || lower_msg.contains("信箱")
                || lower_msg.contains("轉寄")
            {
                let _ =
                    OnboardingService::log_activity(pool, &user.id, "ask_forwarding_info").await;
            }

            let rule_capture = capture_rule_from_chat(
                pool,
                Some(&state.ai_client),
                &user.id,
                &message,
                Some(&user.rule_label_mode),
            )
            .await;
            if let Some(notice) = rule_capture_notice(&user.preferred_language, &rule_capture) {
                res.content = format!("{}\n\n{}", res.content, notice);
            }

            // Append Onboarding Question if needed
            if user.onboarding_step < 4 {
                if let Some(question) = OnboardingService::get_next_onboarding_question(&user).await
                {
                    res.content = format!("{}\n\n---\n💡 [Onboarding] {}", res.content, question);
                }
                // Advance onboarding step
                sqlx::query("UPDATE users SET onboarding_step = onboarding_step + 1 WHERE id = ?")
                    .bind(&user.id)
                    .execute(pool)
                    .await
                    .ok();
            }

            let ai_client_clone = state.ai_client.clone();
            let pool_clone = state.pool.clone();
            let user_id = user.id.clone();
            let msg_clone = message.clone();
            let reply_clone = res.content.clone();
            tokio::spawn(async move {
                let _ = OnboardingService::update_memory(
                    &ai_client_clone,
                    &pool_clone,
                    &user_id,
                    &msg_clone,
                    &reply_clone,
                )
                .await;
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
            let docs_context =
                build_docs_context(&message, None, &state.config.docs_whitelist).await;
            OnboardingService::generate_anonymous_reply(
                &state.ai_client,
                &message,
                guest_name,
                &state.config.assistant_email,
                docs_context.clone(),
            )
            .await
            .unwrap_or_else(|_| crate::ai::ChatResult {
                content: "Error connecting to AI.".to_string(),
                total_tokens: 0,
                duration_ms: 0,
                finish_reason: None,
            })
        }
    } else {
        let docs_context = build_docs_context(&message, None, &state.config.docs_whitelist).await;
        OnboardingService::generate_anonymous_reply(
            &state.ai_client,
            &message,
            guest_name,
            &state.config.assistant_email,
            docs_context.clone(),
        )
        .await
        .unwrap_or_else(|_| crate::ai::ChatResult {
            content: "Error connecting to AI.".to_string(),
            total_tokens: 0,
            duration_ms: 0,
            finish_reason: None,
        })
    };

    // Record this AI reply in chat_logs
    let log_email = if email.is_empty() {
        None
    } else {
        Some(email.clone())
    };
    sqlx::query("INSERT INTO chat_logs (user_email) VALUES (?)")
        .bind(&log_email)
        .execute(pool)
        .await
        .ok();

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
                let suggestion_text = payload
                    .suggestion
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
                let _ = send_system_email_as_assistant(
                    &state,
                    &admin_email,
                    &subject,
                    &text_body,
                    &html_body,
                )
                .await;
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

    let feedback_target: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT user_email, suggestion FROM chat_feedback WHERE id = ?")
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
        let _ =
            send_system_email_as_assistant(&state, &user_email, subject, &text_body, &html_body)
                .await;
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
    readonly_mode_enabled: bool,
    readonly_base: Option<String>,
    overlay_dir: Option<String>,
    remote_debug_sshfs_enabled: bool,
    remote_debug_mode: String,
    remote_debug_remote: Option<String>,
    remote_debug_mount_point: Option<String>,
    remote_debug_overlay_dir: Option<String>,
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
        readonly_mode_enabled: state.config.readonly_mode_enabled,
        readonly_base: state.config.readonly_base.clone(),
        overlay_dir: state.config.overlay_dir.clone(),
        remote_debug_sshfs_enabled: state.config.remote_debug_sshfs_enabled,
        remote_debug_mode: state.config.remote_debug_mode.clone(),
        remote_debug_remote: state.config.remote_debug_remote.clone(),
        remote_debug_mount_point: state.config.remote_debug_mount_point.clone(),
        remote_debug_overlay_dir: state.config.remote_debug_overlay_dir.clone(),
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

use lettre::message::{Message, SinglePart};
use lettre::{SmtpTransport, Transport};
use mail_send::{mail_builder::MessageBuilder, SmtpClientBuilder};

/// Extract pure email address from formats like "Name <email@domain.com>" or "email@domain.com".
/// Returns only the email address part for use in SMTP From field.
fn extract_pure_email(mailbox: &str) -> String {
    let mailbox = mailbox.trim();
    // Check if format is "Name <email@domain.com>"
    if let Some(start) = mailbox.find('<') {
        if let Some(end) = mailbox.find('>') {
            if start < end {
                let email = mailbox[start + 1..end].trim();
                return email.to_string();
            }
        }
    }
    // Otherwise assume it's already a pure email address
    mailbox.to_string()
}

async fn send_system_email_as_assistant(
    state: &AppState,
    to_email: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> bool {
    let preferred = sqlx::query_scalar::<_, String>(
        "SELECT mail_send_method FROM users WHERE email = ? LIMIT 1",
    )
    .bind(to_email)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "direct_mx".to_string());

    match deliver_email_with_fallback(state, to_email, subject, text_body, html_body, &preferred)
        .await
    {
        Ok(_) => true,
        Err(e) => {
            tracing::error!("System email delivery failed to {}: {}", to_email, e);
            false
        }
    }
}

async fn send_via_smtp_relay(
    state: &AppState,
    to_email: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
) -> Result<(), String> {
    let smtp_ready = state
        .config
        .smtp_relay_host
        .as_deref()
        .map(|h| !h.is_empty() && h != "smtp.your-server-address")
        .unwrap_or(false);
    if !smtp_ready {
        return Err("smtp relay not configured".to_string());
    }

    let host = state.config.smtp_relay_host.clone().unwrap_or_default();
    let port = state.config.smtp_relay_port;
    let smtp_user = state.config.smtp_relay_user.clone().unwrap_or_default();
    let pass = state.config.smtp_relay_pass.clone().unwrap_or_default();
    let from_addr = extract_pure_email(&state.config.assistant_email);

    let message = MessageBuilder::new()
        .from(from_addr.as_str())
        .reply_to(from_addr.as_str())
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
        Ok(mut client) => client
            .send(message)
            .await
            .map_err(|e| format!("relay send failed: {:?}", e)),
        Err(e) => Err(format!("relay connect failed: {:?}", e)),
    }
}

async fn send_via_direct_mx(
    state: &AppState,
    to_email: &str,
    subject: &str,
    text_body: &str,
) -> Result<(), String> {
    let from_addr: lettre::message::Mailbox = extract_pure_email(&state.config.assistant_email)
        .parse()
        .unwrap_or_else(|_| "noreply@example.com".parse().expect("default mailbox"));
    let to_addr: lettre::message::Mailbox = to_email
        .parse()
        .map_err(|e| format!("invalid recipient mailbox: {:?}", e))?;

    let domain = to_email.split('@').nth(1).unwrap_or("").trim();
    if domain.is_empty() {
        return Err("recipient domain missing".to_string());
    }

    let Some(mx) = lookup_mx_host(domain).await else {
        return Err(format!("mx lookup failed for domain {}", domain));
    };

    let email_msg = Message::builder()
        .from(from_addr)
        .to(to_addr)
        .subject(subject)
        .singlepart(SinglePart::plain(text_body.to_string()))
        .map_err(|e| format!("direct message build failed: {:?}", e))?;

    let mailer = SmtpTransport::relay(&mx)
        .map_err(|e| format!("direct transport build failed: {:?}", e))?
        .port(25)
        .build();

    mailer
        .send(&email_msg)
        .map_err(|e| format!("direct mx send failed: {:?}", e))?;
    Ok(())
}

async fn deliver_email_with_fallback(
    state: &AppState,
    to_email: &str,
    subject: &str,
    text_body: &str,
    html_body: &str,
    preferred_method: &str,
) -> Result<(), String> {
    let methods = if preferred_method == "relay" {
        vec!["relay", "direct_mx"]
    } else {
        vec!["direct_mx", "relay"]
    };

    let mut errors = Vec::new();

    for method in methods {
        let result = match method {
            "relay" => send_via_smtp_relay(state, to_email, subject, text_body, html_body).await,
            _ => send_via_direct_mx(state, to_email, subject, text_body).await,
        };

        match result {
            Ok(_) => return Ok(()),
            Err(e) => errors.push(format!("{}: {}", method, e)),
        }
    }

    Err(errors.join(" | "))
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
         ON CONFLICT(email) DO UPDATE SET magic_token = excluded.magic_token",
    )
    .bind(&new_id)
    .bind(&email)
    .bind(&token)
    .execute(&state.pool)
    .await;

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

    let mut subject = "Your AI Mail Butler Login Link".to_string();
    let mut plain_text = format!(
        "Welcome to AI Mail Butler!\n\nClick the link below to securely login without a password:\n{}",
        login_url
    );
    let user_pref: (String, String, String) = sqlx::query_as(
        "SELECT email_format, preferred_language, mail_send_method FROM users WHERE email = ?",
    )
    .bind(&email)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(Some((
        "both".to_string(),
        "en".to_string(),
        "direct_mx".to_string(),
    )))
    .unwrap_or((
        "both".to_string(),
        "en".to_string(),
        "direct_mx".to_string(),
    ));

    let email_format = user_pref.0;
    let preferred_language = user_pref.1;
    let preferred_send_method = user_pref.2;

    if preferred_language == "zh-TW" {
        subject = "您的 AI 郵件助理登入連結".to_string();
        plain_text = format!(
            "歡迎使用 AI Mail Butler！\n\n請點擊下方連結安全無密碼登入：\n{}",
            login_url
        );
    }

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

    let (final_text, final_html) = match email_format.as_str() {
        "html" => ("".to_string(), html_body),
        "plain" => (plain_text.clone(), "".to_string()),
        _ => (plain_text.clone(), html_body),
    };

    match deliver_email_with_fallback(
        &state,
        &email,
        subject.as_str(),
        final_text.as_str(),
        final_html.as_str(),
        &preferred_send_method,
    )
    .await
    {
        Ok(_) => Json(
            serde_json::json!({ "status": "success", "message": "Magic link sent to your email" }),
        ),
        Err(err_msg) => {
            tracing::error!(
                "Magic link delivery failed: {}. Login URL is in server console.",
                err_msg
            );
            let user_id = get_user_id_by_email(&state.pool, &email).await;
            log_mail_event(
                &state.pool,
                "ERROR",
                "smtp_send",
                &err_msg,
                Some(&email),
                user_id.as_deref(),
            )
            .await;
            Json(
                serde_json::json!({ "status": "ok_debug", "message": "Email delivery failed; login URL logged to console" }),
            )
        }
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
        .await
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                tracing::warn!("dig returned empty results for MX lookup of {}", domain);
                return None;
            }
            parse_mx_records(&stdout)
        }
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
                if !host.is_empty() {
                    Some((priority, host))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    records.sort_by_key(|(p, _)| *p);
    records.into_iter().next().map(|(_, host)| host)
}

#[cfg(test)]
mod web_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn find_header_end(buf: &[u8]) -> Option<usize> {
        buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
    }

    async fn start_mock_ai_server() -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock ai listener");
        let addr = listener.local_addr().expect("mock ai local addr");

        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept mock ai conn");

            let mut buf = Vec::new();
            let mut tmp = [0_u8; 1024];
            let mut header_end = None;
            let mut content_len = 0_usize;

            loop {
                let n = socket.read(&mut tmp).await.expect("read mock ai request");
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);

                if header_end.is_none() {
                    if let Some(h) = find_header_end(&buf) {
                        header_end = Some(h);
                        let headers = String::from_utf8_lossy(&buf[..h]);
                        for line in headers.lines() {
                            if let Some((name, val)) = line.split_once(':') {
                                if name.trim().eq_ignore_ascii_case("content-length") {
                                    content_len = val.trim().parse::<usize>().unwrap_or(0);
                                }
                            }
                        }
                    }
                }

                if let Some(h) = header_end {
                    if buf.len() >= h + content_len {
                        break;
                    }
                }
            }

            let h = header_end.expect("request header end");
            let body_bytes = &buf[h..h + content_len];
            let body = String::from_utf8_lossy(body_bytes);
            let from_stored = body.contains("STORED_MARKER_ONLY");

            let reason = if from_stored {
                "from_stored"
            } else {
                "from_preview"
            };

            let ai_content = format!(
                "[{{\"reason\":\"{}\",\"amount\":12.5,\"category\":\"expense\",\"direction\":\"expense\",\"currency\":\"TWD\"}}]",
                reason
            );
            let payload = serde_json::json!({
                "choices": [{
                    "message": {"content": ai_content},
                    "finish_reason": "stop"
                }],
                "usage": {
                    "total_tokens": 1,
                    "completion_tokens": 1,
                    "prompt_tokens": 1
                }
            })
            .to_string();

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                payload.len(),
                payload
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write mock ai response");
        });

        (format!("http://{}", addr), handle)
    }

    #[test]
    fn looks_like_email_rule_detects_common_intents() {
        assert!(looks_like_email_rule("請幫我把帳單通知都轉寄並提醒我"));
        assert!(looks_like_email_rule(
            "When I receive invoice emails, remind me to reply quickly"
        ));
        assert!(looks_like_email_rule(
            "Please add rule: remind me when invoice arrives"
        ));
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
        assert!(!is_doc_allowed("RBAC.md", &["gmail".to_string()]));

        assert!(
            language_bonus("RBAC.zh-TW.md", Some("zh-TW"))
                > language_bonus("RBAC.md", Some("zh-TW"))
        );
        assert!(
            language_bonus("RBAC.md", Some("en")) > language_bonus("RBAC.zh-TW.md", Some("en"))
        );
    }

    #[test]
    fn sanitize_path_component_normalizes_unsafe_chars() {
        assert_eq!(sanitize_path_component("A/B C?.txt"), "A_B_C_.txt");
        assert_eq!(sanitize_path_component("<>"), "__");
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
        assert_eq!(
            parse_training_consent_answer("No, I do not consent"),
            Some(false)
        );
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
                rule_label TEXT NOT NULL DEFAULT 'RULE',
                source TEXT NOT NULL,
                is_enabled INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create email_rules");

        let user_id = "u1";
        let msg = "forward invoices to me and remind me";
        let first =
            capture_rule_from_chat(&pool, None, user_id, msg, Some("deterministic_only")).await;
        let second =
            capture_rule_from_chat(&pool, None, user_id, msg, Some("deterministic_only")).await;

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
                rule_label TEXT NOT NULL DEFAULT 'RULE',
                source TEXT NOT NULL,
                is_enabled INTEGER NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .expect("create email_rules");

        let outcome =
            capture_rule_from_chat(&pool, None, "u1", "hello there", Some("deterministic_only"))
                .await;
        assert!(matches!(outcome, RuleCaptureOutcome::None));

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM email_rules")
            .fetch_one(&pool)
            .await
            .expect("count rows");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn manual_process_emails_prefers_stored_content_over_preview() {
        let (mock_ai_url, mock_ai_task) = start_mock_ai_server().await;
        let old_ai_base = std::env::var("AI_API_BASE_URL").ok();
        std::env::set_var("AI_API_BASE_URL", &mock_ai_url);

        let pool = crate::db::connect("sqlite::memory:")
            .await
            .expect("create schema");

        let user_id = uuid::Uuid::new_v4().to_string();
        let email_id = uuid::Uuid::new_v4().to_string();

        sqlx::query("INSERT INTO users (id, email, is_onboarded, role) VALUES (?, ?, 1, 'user')")
            .bind(&user_id)
            .bind("test@example.com")
            .execute(&pool)
            .await
            .expect("insert user");

        sqlx::query("INSERT INTO emails (id, user_id, subject, preview, stored_content, status) VALUES (?, ?, ?, ?, ?, 'pending')")
            .bind(&email_id)
            .bind(&user_id)
            .bind("Statement")
            .bind("PREVIEW_MARKER_ONLY")
            .bind("STORED_MARKER_ONLY")
            .execute(&pool)
            .await
            .expect("insert email");

        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            server_port: 3000,
            ai_api_key: String::new(),
            developer_email: None,
            smtp_relay_host: None,
            smtp_relay_port: 587,
            smtp_relay_user: None,
            smtp_relay_pass: None,
            assistant_email: "assistant@example.com".to_string(),
            docs_whitelist: vec![],
            readonly_mode_enabled: false,
            readonly_base: None,
            overlay_dir: None,
            remote_debug_sshfs_enabled: false,
            remote_debug_mode: "readonly".to_string(),
            remote_debug_remote: None,
            remote_debug_mount_point: None,
            remote_debug_overlay_dir: None,
        };

        let state = AppState {
            pool: pool.clone(),
            ai_client: crate::ai::AiClient::new(&config),
            admin_email: None,
            developer_email: None,
            config: std::sync::Arc::new(config),
        };

        let payload = ManualProcessRequest {
            email: "test@example.com".to_string(),
            email_ids: vec![email_id.clone()],
            force_reextract: Some(true),
        };

        let Json(resp) = post_manual_process_emails(State(state), Json(payload)).await;
        assert_eq!(resp["status"], "success");
        assert_eq!(resp["processed"], 1);

        let reason: Option<String> = sqlx::query_scalar(
            "SELECT reason FROM email_financial_records WHERE user_id = ? AND email_id = ? ORDER BY created_at DESC LIMIT 1"
        )
        .bind(&user_id)
        .bind(&email_id)
        .fetch_optional(&pool)
        .await
        .expect("query extracted reason");
        assert_eq!(reason.as_deref(), Some("from_stored"));

        mock_ai_task.await.expect("mock ai task done");

        match old_ai_base {
            Some(v) => std::env::set_var("AI_API_BASE_URL", v),
            None => std::env::remove_var("AI_API_BASE_URL"),
        }
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
    tracing::info!(
        ">>> [AUTH] Attempting to verify token: '{}'",
        mask_token(token)
    );

    // Debug: check total tokens in DB
    let total_tokens: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE magic_token IS NOT NULL")
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0);
    tracing::info!(
        ">>> [AUTH] System current has {} active magic tokens in DB.",
        total_tokens
    );

    let query_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE magic_token = ?")
        .bind(token)
        .fetch_optional(&state.pool)
        .await;

    match query_result {
        Ok(Some(mut u)) => {
            tracing::info!(
                ">>> [AUTH] SUCCESS: Token match found for user: {}",
                u.email
            );
            // Clear token after use
            let clear_res = sqlx::query("UPDATE users SET magic_token = NULL WHERE id = ?")
                .bind(&u.id)
                .execute(&state.pool)
                .await;

            if let Err(e) = clear_res {
                tracing::error!(
                    ">>> [AUTH] FAILED to clear token for user {}: {:?}",
                    u.email,
                    e
                );
            }

            u.magic_token = None;
            u.role = role_for_email(&state, &u.email);
            Json(Some(u))
        }
        Ok(None) => {
            tracing::warn!(
                ">>> [AUTH] FAILED: No user found with token '{}'.",
                mask_token(token)
            );
            Json(None)
        }
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
    mail_send_method: Option<String>,
    rule_label_mode: Option<String>,
    training_data_consent: bool,
    timezone: Option<String>,
    preferred_language: Option<String>,
    display_name: Option<String>,
    assistant_name_zh: Option<String>,
    assistant_name_en: Option<String>,
    assistant_tone_zh: Option<String>,
    assistant_tone_en: Option<String>,
    pdf_passwords: Option<Vec<String>>,
    time_format: Option<String>,
    date_format: Option<String>,
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
struct DeleteRuleRequest {
    email: String,
    id: i64,
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
    let pdf_passwords_json = payload
        .pdf_passwords
        .as_ref()
        .and_then(|v| serde_json::to_string(v).ok());
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
    let mail_send_method = payload
        .mail_send_method
        .as_deref()
        .map(str::trim)
        .filter(|v| *v == "direct_mx" || *v == "relay")
        .unwrap_or("direct_mx")
        .to_string();
    let rule_label_mode = payload
        .rule_label_mode
        .as_deref()
        .map(str::trim)
        .filter(|v| *v == "ai_first" || *v == "deterministic_only")
        .unwrap_or("ai_first")
        .to_string();
    let time_format = payload
        .time_format
        .as_deref()
        .map(str::trim)
        .filter(|v| *v == "24h" || *v == "12h")
        .unwrap_or("24h")
        .to_string();
    let date_format = payload
        .date_format
        .as_deref()
        .map(str::trim)
        .filter(|v| matches!(*v, "auto" | "iso" | "us" | "eu" | "tw"))
        .unwrap_or("auto")
        .to_string();

    let result = sqlx::query("UPDATE users SET auto_reply = ?, dry_run = ?, email_format = ?, mail_send_method = ?, rule_label_mode = ?, training_data_consent = ?, \
                              training_consent_updated_at = CASE WHEN training_data_consent != ? THEN CURRENT_TIMESTAMP ELSE training_consent_updated_at END, \
                              timezone = ?, preferred_language = ?, display_name = ?, assistant_name_zh = ?, assistant_name_en = ?, assistant_tone_zh = ?, assistant_tone_en = ?, pdf_passwords = ?, time_format = ?, date_format = ? WHERE email = ?")
        .bind(payload.auto_reply)
        .bind(payload.dry_run)
        .bind(&payload.email_format)
        .bind(&mail_send_method)
        .bind(&rule_label_mode)
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
        .bind(&time_format)
        .bind(&date_format)
        .bind(&payload.email)
        .execute(&state.pool).await;

    if result.is_ok() {
        let user_id = &payload.email; // Using email as user_id for stats for now
        let _ = OnboardingService::log_activity(&state.pool, user_id, "change_settings").await;
        if payload.auto_reply {
            let _ =
                OnboardingService::log_activity(&state.pool, user_id, "enable_auto_reply").await;
        }
        if payload.dry_run {
            let _ = OnboardingService::log_activity(&state.pool, user_id, "enable_dry_run").await;
        }
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
    let is_admin = query
        .email
        .as_deref()
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

async fn post_retry_mail_error(
    State(state): State<AppState>,
    Json(payload): Json<RetryMailErrorRequest>,
) -> Json<serde_json::Value> {
    let requester_email = payload.email.trim().to_lowercase();
    if requester_email.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    }

    let requester_user_id = get_user_id_by_email(&state.pool, &requester_email).await;
    let is_privileged = is_admin_or_developer(&state, &requester_email);

    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>)>(
        "SELECT error_type, context, user_id, message FROM mail_errors WHERE id = ?",
    )
    .bind(payload.error_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some((error_type, context, error_user_id, _message)) = row else {
        return Json(serde_json::json!({ "status": "error", "message": "Error log not found" }));
    };

    if !is_privileged {
        let Some(uid) = requester_user_id.as_ref() else {
            return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
        };
        if error_user_id.as_deref() != Some(uid.as_str()) {
            return Json(serde_json::json!({ "status": "error", "message": "Unauthorized" }));
        }
    }

    if error_type != "unknown_sender" {
        return Json(
            serde_json::json!({ "status": "error", "message": "Only unknown_sender logs can be retried" }),
        );
    }

    let Some(context_path) = context else {
        return Json(
            serde_json::json!({ "status": "error", "message": "Missing log context path" }),
        );
    };

    let Some(source_path) = resolve_mail_error_source_path(&state, &context_path) else {
        return Json(
            serde_json::json!({ "status": "error", "message": "Cannot locate original .eml file" }),
        );
    };

    let file_name = source_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("mail.eml");
    let retry_name = format!(
        "retry_{}_{}",
        chrono::Utc::now().timestamp_millis(),
        file_name
    );
    let retry_spool_root = resolve_runtime_mail_path(&state, "data/mail_spool");
    let retry_path = retry_spool_root.join(retry_name);

    if let Err(e) = fs::copy(&source_path, &retry_path).await {
        return Json(serde_json::json!({
            "status": "error",
            "message": format!("Failed to queue retry file: {}", e)
        }));
    }

    log_mail_event(
        &state.pool,
        "INFO",
        "manual_retry",
        &format!(
            "Queued retry for mail_errors.id={} from {}",
            payload.error_id,
            source_path.display()
        ),
        Some(&retry_path.to_string_lossy()),
        requester_user_id.as_deref(),
    )
    .await;

    Json(serde_json::json!({
        "status": "success",
        "message": "Retry queued. The spool worker will re-check Delivered-To / X-Original-To and candidate owner headers.",
        "queued_path": retry_path.to_string_lossy(),
    }))
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
        "SELECT id, user_id, rule_text, rule_label, source, is_enabled, matched_count, CAST(created_at AS TEXT) as created_at, CAST(updated_at AS TEXT) as updated_at \
         FROM email_rules WHERE user_id = ? ORDER BY is_enabled DESC, updated_at DESC"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "status": "success", "rules": rules }))
}

async fn post_manual_process_emails(
    State(state): State<AppState>,
    Json(payload): Json<ManualProcessRequest>,
) -> Json<serde_json::Value> {
    if payload.email.trim().is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    }
    if payload.email_ids.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "No email ids provided" }));
    }

    let Some(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let force_reextract = payload.force_reextract.unwrap_or(false);
    let mut processed = 0_i64;
    let mut skipped = 0_i64;
    let mut failed = 0_i64;
    let mut results = Vec::new();

    for email_id in payload.email_ids {
        let row = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<String>, String)>(
            "SELECT id, subject, preview, stored_content, status FROM emails WHERE id = ? AND user_id = ?"
        )
        .bind(&email_id)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

        let Some((id, subject, preview, stored_content, status)) = row else {
            failed += 1;
            results.push(serde_json::json!({ "email_id": email_id, "result": "failed", "reason": "Email not found" }));
            continue;
        };

        if status != "pending" && !force_reextract {
            skipped += 1;
            results.push(serde_json::json!({ "email_id": id, "result": "skipped", "reason": "Status is not pending" }));
            continue;
        }

        if force_reextract {
            let _ = sqlx::query(
                "DELETE FROM email_financial_records WHERE user_id = ? AND email_id = ?",
            )
            .bind(&user.id)
            .bind(&id)
            .execute(&state.pool)
            .await;
            let _ = sqlx::query("DELETE FROM auto_replies WHERE user_id = ? AND source_email_id = ? AND reply_status = 'draft'")
                .bind(&user.id)
                .bind(&id)
                .execute(&state.pool)
                .await;
        }

        let source_text = stored_content
            .filter(|s| !s.trim().is_empty())
            .or(preview)
            .unwrap_or_default();
        crate::mail::analyze_and_store_financial_records(
            &state.pool,
            &state.ai_client,
            &user,
            &id,
            subject.as_deref().unwrap_or("(no subject)"),
            &source_text,
        )
        .await;

        let _ = sqlx::query("UPDATE emails SET status = 'drafted' WHERE id = ?")
            .bind(&id)
            .execute(&state.pool)
            .await;

        processed += 1;
        results.push(serde_json::json!({ "email_id": id, "result": "processed" }));
    }

    Json(serde_json::json!({
        "status": "success",
        "processed": processed,
        "skipped": skipped,
        "failed": failed,
        "results": results,
    }))
}

async fn get_finance_records(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rows = sqlx::query_as::<_, FinanceRecordRow>(
        "SELECT id, email_id, subject, reason, category, direction, amount, currency, month_key, month_total_after, finance_type, due_date, statement_amount, issuing_bank, card_last4, transaction_month_key, CAST(created_at AS TEXT) as created_at \
         FROM email_financial_records WHERE user_id = ? ORDER BY created_at DESC LIMIT 500"
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "status": "success", "records": rows }))
}

async fn get_finance_monthly(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rows = sqlx::query_as::<_, MonthlyFinanceSummaryRow>(
        "SELECT month_key, category, total_amount, CAST(updated_at AS TEXT) as updated_at \
         FROM monthly_finance_summary WHERE user_id = ? ORDER BY month_key DESC, category ASC",
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({ "status": "success", "monthly": rows }))
}

async fn post_create_rule(
    State(state): State<AppState>,
    Json(payload): Json<CreateRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let generated_label = generate_rule_label_with_ai(
        Some(&state.ai_client),
        rule_text,
        Some(&user.rule_label_mode),
    )
    .await;
    let result = sqlx::query(
        "INSERT INTO email_rules (user_id, rule_text, rule_label, source, is_enabled) VALUES (?, ?, ?, 'manual', 1)"
    )
    .bind(&user.id)
    .bind(rule_text)
    .bind(generated_label)
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
    let Some(user) = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&payload.email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let rule_text = payload.rule_text.trim();
    if rule_text.is_empty() {
        return Json(serde_json::json!({ "status": "error", "message": "Rule cannot be empty" }));
    }

    let generated_label = generate_rule_label_with_ai(
        Some(&state.ai_client),
        rule_text,
        Some(&user.rule_label_mode),
    )
    .await;
    let result = sqlx::query(
        "UPDATE email_rules SET rule_text = ?, rule_label = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ? AND user_id = ?"
    )
    .bind(rule_text)
    .bind(generated_label)
    .bind(payload.id)
    .bind(&user.id)
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

async fn post_delete_rule(
    State(state): State<AppState>,
    Json(payload): Json<DeleteRuleRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let result = sqlx::query("DELETE FROM email_rules WHERE id = ? AND user_id = ?")
        .bind(payload.id)
        .bind(&user_id)
        .execute(&state.pool)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({ "status": "success" })),
        Ok(_) => Json(serde_json::json!({ "status": "error", "message": "Rule not found" })),
        Err(e) => {
            tracing::error!("Failed to delete rule: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Delete failed" }))
        }
    }
}

#[derive(Serialize)]
struct AutoReplyResponse {
    id: String,
    source_email_id: Option<String>,
    from: String,
    subject: String,
    body: String,
}

async fn get_auto_replies(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let Some(email) = query.email else {
        return Json(serde_json::json!({ "status": "error", "message": "Missing email" }));
    };

    let Some(user_id) = get_user_id_by_email(&state.pool, &email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    match crate::services::EmailReplyService::get_draft_replies(&state.pool, &user_id).await {
        Ok(drafts) => {
            let replies: Vec<AutoReplyResponse> = drafts
                .into_iter()
                .map(
                    |(id, source_email_id, from, subject, body)| AutoReplyResponse {
                        id,
                        source_email_id,
                        from,
                        subject,
                        body,
                    },
                )
                .collect();
            Json(serde_json::json!({ "status": "success", "replies": replies }))
        }
        Err(e) => {
            tracing::error!("Failed to fetch auto-replies: {:?}", e);
            Json(serde_json::json!({ "status": "error", "message": "Failed to fetch replies" }))
        }
    }
}

#[derive(Deserialize)]
struct SendAutoReplyRequest {
    email: String,
    reply_id: String,
}

#[derive(Deserialize)]
struct UpdateAutoReplyRequest {
    email: String,
    reply_id: String,
    reply_body: String,
}

async fn post_update_auto_reply(
    State(state): State<AppState>,
    Json(payload): Json<UpdateAutoReplyRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let body = payload.reply_body.trim();
    if body.is_empty() {
        return Json(
            serde_json::json!({ "status": "error", "message": "Reply body cannot be empty" }),
        );
    }

    let result = sqlx::query(
        "UPDATE auto_replies SET reply_body = ? WHERE id = ? AND user_id = ? AND reply_status = 'draft'"
    )
    .bind(body)
    .bind(&payload.reply_id)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(serde_json::json!({ "status": "success" })),
        Ok(_) => Json(serde_json::json!({ "status": "error", "message": "Draft not found" })),
        Err(e) => {
            tracing::error!("Failed to update auto-reply draft: {}", e);
            Json(serde_json::json!({ "status": "error", "message": "Update failed" }))
        }
    }
}

async fn post_send_auto_reply(
    State(state): State<AppState>,
    Json(payload): Json<SendAutoReplyRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    // Fetch draft reply
    let draft: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT reply_body, original_from, source_email_id FROM auto_replies WHERE id = ? AND user_id = ? AND reply_status = 'draft'"
    )
    .bind(&payload.reply_id)
    .bind(&user_id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some((reply_body, recipient, source_email_id)) = draft else {
        return Json(serde_json::json!({ "status": "error", "message": "Draft not found" }));
    };

    let to_addr = recipient.clone();
    let subject = "Re: Auto-Reply";
    let preferred_send_method =
        sqlx::query_scalar::<_, String>("SELECT mail_send_method FROM users WHERE id = ? LIMIT 1")
            .bind(&user_id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "direct_mx".to_string());

    match deliver_email_with_fallback(
        &state,
        &to_addr,
        subject,
        &reply_body,
        &reply_body,
        &preferred_send_method,
    )
    .await
    {
        Ok(_) => {
            let _ = sqlx::query("UPDATE auto_replies SET reply_status = 'sent', sent_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(&payload.reply_id)
                .execute(&state.pool)
                .await;
            if let Some(email_id) = source_email_id {
                let _ = sqlx::query(
                    "UPDATE emails SET status = 'replied' WHERE id = ? AND user_id = ?",
                )
                .bind(email_id)
                .bind(&user_id)
                .execute(&state.pool)
                .await;
            }
            Json(serde_json::json!({ "status": "success", "message": "Reply sent" }))
        }
        Err(e) => {
            tracing::error!("Auto-reply delivery failed: {}", e);
            log_mail_event(
                &state.pool,
                "ERROR",
                "smtp_send",
                &e,
                Some(&payload.reply_id),
                Some(&user_id),
            )
            .await;
            Json(serde_json::json!({ "status": "error", "message": "Failed to send reply" }))
        }
    }
}

#[derive(Deserialize)]
struct DeleteAutoReplyRequest {
    email: String,
    reply_id: String,
}

async fn post_delete_auto_reply(
    State(state): State<AppState>,
    Json(payload): Json<DeleteAutoReplyRequest>,
) -> Json<serde_json::Value> {
    let Some(user_id) = get_user_id_by_email(&state.pool, &payload.email).await else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    match sqlx::query("DELETE FROM auto_replies WHERE id = ? AND user_id = ?")
        .bind(&payload.reply_id)
        .bind(&user_id)
        .execute(&state.pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                Json(serde_json::json!({ "status": "success", "message": "Reply deleted" }))
            } else {
                Json(serde_json::json!({ "status": "error", "message": "Reply not found" }))
            }
        }
        Err(e) => {
            tracing::error!("Failed to delete auto-reply: {:?}", e);
            Json(serde_json::json!({ "status": "error", "message": "Delete failed" }))
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

    let preferred_send_method = sqlx::query_scalar::<_, String>(
        "SELECT mail_send_method FROM users WHERE email = ? LIMIT 1",
    )
    .bind(user_email)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten()
    .unwrap_or_else(|| "direct_mx".to_string());

    deliver_email_with_fallback(
        state,
        user_email,
        subject,
        &text_body,
        &html_body,
        &preferred_send_method,
    )
    .await
}

async fn post_request_data_deletion(
    State(state): State<AppState>,
    Json(payload): Json<DataDeletionRequestPayload>,
) -> Json<serde_json::Value> {
    let email = payload.email.trim().to_ascii_lowercase();
    let user_row: Option<(String, String)> =
        sqlx::query_as("SELECT id, preferred_language FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or(None);

    let Some((user_id, preferred_language)) = user_row else {
        return Json(serde_json::json!({ "status": "error", "message": "User not found" }));
    };

    let sender_dir = resolve_runtime_mail_path(
        &state,
        &format!(
            "data/mail_spool/{}",
            sanitize_path_component(&email.to_ascii_lowercase())
        ),
    );
    let sender_dir_str = sender_dir.to_string_lossy().to_string();
    let snapshot = build_user_data_snapshot(
        &state.pool,
        &user_id,
        &email,
        &sender_dir_str,
        &state.config,
    )
    .await;
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

    let send_result = send_data_deletion_confirmation_email(
        &state,
        &email,
        &preferred_language,
        &snapshot,
        &confirm_url,
    )
    .await;
    let delivered = send_result.is_ok();
    if let Err(reason) = send_result {
        let msg = format!(
            "Failed to deliver data deletion confirmation email to {}: {}",
            email, reason
        );
        tracing::error!("{}", msg);
        let context = format!("confirm_url={}; reason={}", confirm_url, reason);
        log_mail_event(
            &state.pool,
            "ERROR",
            "gdpr_email_send",
            &msg,
            Some(&context),
            Some(&user_id),
        )
        .await;
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
         WHERE r.token = ?",
    )
    .bind(query.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(
            serde_json::json!({ "status": "error", "message": "Invalid or expired token" }),
        );
    };

    if row.status == "finalized" {
        return Json(
            serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }),
        );
    }

    if row.status == "requested" {
        let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'email_confirmed', email_confirmed_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&row.id)
            .execute(&state.pool)
            .await;
    }

    let snapshot: DataDeletionSnapshot =
        serde_json::from_str(&row.snapshot_json).unwrap_or_default();

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
        return Json(
            serde_json::json!({ "status": "error", "message": "Final confirmation is required" }),
        );
    }

    let row = sqlx::query_as::<_, DataDeletionRequestRow>(
        "SELECT r.id, r.user_id, u.email as user_email, r.status, r.snapshot_json \
         FROM data_deletion_requests r \
         JOIN users u ON u.id = r.user_id \
         WHERE r.token = ?",
    )
    .bind(payload.token.trim())
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let Some(row) = row else {
        return Json(
            serde_json::json!({ "status": "error", "message": "Invalid or expired token" }),
        );
    };

    if row.status == "finalized" {
        return Json(
            serde_json::json!({ "status": "finalized", "message": "Deletion already completed" }),
        );
    }

    let sender_dir = resolve_runtime_mail_path(
        &state,
        &format!(
            "data/mail_spool/{}",
            sanitize_path_component(&row.user_email.to_ascii_lowercase())
        ),
    );

    let mut tx = match state.pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return Json(
                serde_json::json!({ "status": "error", "message": format!("Cannot start deletion transaction: {}", e) }),
            );
        }
    };

    let _ = sqlx::query("DELETE FROM emails WHERE user_id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM email_rules WHERE user_id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM user_memories WHERE user_id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM user_activity_stats WHERE user_id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM mail_errors WHERE user_id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM chat_logs WHERE user_email = ?")
        .bind(&row.user_email)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM chat_transcripts WHERE user_email = ?")
        .bind(&row.user_email)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM chat_feedback WHERE user_email = ?")
        .bind(&row.user_email)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("UPDATE data_deletion_requests SET status = 'finalized', finalized_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(&row.id)
        .execute(&mut *tx)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(&row.user_id)
        .execute(&mut *tx)
        .await;

    if let Err(e) = tx.commit().await {
        return Json(
            serde_json::json!({ "status": "error", "message": format!("Deletion failed: {}", e) }),
        );
    }

    let _ = fs::remove_dir_all(&sender_dir).await;

    Json(serde_json::json!({
        "status": "success",
        "message": "All removable user data has been deleted. Cached/overwritten database pages are not recoverable.",
    }))
}

#[derive(Deserialize)]
struct ConsentUpdateRequest {
    email: String,
    consent_type: String,
    consent_granted: bool,
}

async fn post_consent_update(
    State(state): State<AppState>,
    Json(req): Json<ConsentUpdateRequest>,
) -> Json<serde_json::Value> {
    let user_email = req.email.trim().to_lowercase();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }
    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&user_email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    {
        Some(u) => u,
        None => return Json(serde_json::json!({"status": "error", "message": "User not found"})),
    };

    let id = uuid::Uuid::new_v4().to_string();
    let policy_version = "1.0";
    let ip = "0.0.0.0";
    let user_agent = "web";

    let _ = sqlx::query(
        "INSERT INTO consent_audit_trail (id, user_id, policy_version, consent_type, consent_granted, consent_source, ip_address, user_agent) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&user.id)
    .bind(policy_version)
    .bind(&req.consent_type)
    .bind(req.consent_granted)
    .bind("dashboard")
    .bind(ip)
    .bind(user_agent)
    .execute(&state.pool)
    .await;

    let _ = sqlx::query("UPDATE users SET training_data_consent = ?, training_consent_updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(req.consent_granted)
        .bind(&user.id)
        .execute(&state.pool)
        .await;

    Json(serde_json::json!({"status": "success", "message": "Consent updated"}))
}

async fn get_consent_history(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let user_email = params.get("email").cloned().unwrap_or_default();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }

    let history = sqlx::query_as::<_, ConsentAuditTrail>(
        "SELECT id, user_id, policy_version, consent_type, consent_granted, consent_source, ip_address, user_agent, created_at FROM consent_audit_trail WHERE user_id = (SELECT id FROM users WHERE email = ?) ORDER BY created_at DESC LIMIT 50"
    )
    .bind(&user_email)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({"status": "success", "history": history}))
}

#[derive(Deserialize)]
struct DsarRequestInput {
    email: String,
    request_type: String,
}

async fn post_dsar_request(
    State(state): State<AppState>,
    Json(req): Json<DsarRequestInput>,
) -> Json<serde_json::Value> {
    let user_email = req.email.trim().to_lowercase();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }
    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&user_email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    {
        Some(u) => u,
        None => return Json(serde_json::json!({"status": "error", "message": "User not found"})),
    };

    let valid_types = [
        "access",
        "export",
        "correction",
        "restriction",
        "withdraw-consent",
    ];
    if !valid_types.contains(&req.request_type.as_str()) {
        return Json(serde_json::json!({"status": "error", "message": "Invalid request type"}));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let snapshot = serde_json::json!({
        "user_email": &user_email,
        "requested_at": chrono::Utc::now().to_rfc3339(),
    });

    let _ = sqlx::query(
        "INSERT INTO dsar_requests (id, user_id, request_type, status, snapshot_json) VALUES (?, ?, ?, 'pending', ?)"
    )
    .bind(&id)
    .bind(&user.id)
    .bind(&req.request_type)
    .bind(snapshot.to_string())
    .execute(&state.pool)
    .await;

    Json(
        serde_json::json!({"status": "success", "message": "DSAR request created", "request_id": id}),
    )
}

async fn get_dsar_status(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let user_email = params.get("email").cloned().unwrap_or_default();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }

    let requests = sqlx::query_as::<_, DsarRequest>(
        "SELECT id, user_id, request_type, status, admin_notes, completed_at, created_at FROM dsar_requests WHERE user_id = (SELECT id FROM users WHERE email = ?) ORDER BY created_at DESC LIMIT 20"
    )
    .bind(&user_email)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({"status": "success", "requests": requests}))
}

#[derive(Deserialize)]
struct PrivacySettingsInput {
    email: String,
    do_not_sell_share: Option<bool>,
    cross_border_disclosure_given: Option<bool>,
    data_location_preference: Option<String>,
}

async fn post_privacy_settings(
    State(state): State<AppState>,
    Json(req): Json<PrivacySettingsInput>,
) -> Json<serde_json::Value> {
    let user_email = req.email.trim().to_lowercase();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }
    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&user_email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    {
        Some(u) => u,
        None => return Json(serde_json::json!({"status": "error", "message": "User not found"})),
    };

    let mut updates = Vec::new();
    if let Some(v) = req.do_not_sell_share {
        updates.push(format!("do_not_sell_share = {}", v as i32));
    }
    if let Some(v) = req.cross_border_disclosure_given {
        updates.push(format!("cross_border_disclosure_given = {}", v as i32));
    }
    if let Some(ref loc) = req.data_location_preference {
        updates.push(format!("data_location_preference = '{}'", loc));
    }

    if !updates.is_empty() {
        let set_clause = updates.join(", ");
        let _ = sqlx::query(&format!(
            "INSERT INTO user_privacy_settings (user_id, do_not_sell_share, cross_border_disclosure_given, data_location_preference, updated_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP) ON CONFLICT(user_id) DO UPDATE SET {}",
            set_clause
        ))
        .bind(&user.id)
        .bind(req.do_not_sell_share.unwrap_or(false))
        .bind(req.cross_border_disclosure_given.unwrap_or(false))
        .bind(req.data_location_preference.clone().unwrap_or_default())
        .execute(&state.pool)
        .await;
    }

    Json(serde_json::json!({"status": "success", "message": "Privacy settings updated"}))
}

async fn get_privacy_settings(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let user_email = params.get("email").cloned().unwrap_or_default();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }

    let settings = sqlx::query_as::<_, UserPrivacySettings>(
        "SELECT user_id, do_not_sell_share, cross_border_disclosure_given, data_location_preference, updated_at FROM user_privacy_settings WHERE user_id = (SELECT id FROM users WHERE email = ?)"
    )
    .bind(&user_email)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    Json(serde_json::json!({"status": "success", "settings": settings}))
}

#[derive(Deserialize)]
struct AgeVerificationInput {
    email: String,
    is_minor: bool,
    guardian_consent_given: Option<bool>,
    guardian_email: Option<String>,
}

async fn post_age_verification(
    State(state): State<AppState>,
    Json(req): Json<AgeVerificationInput>,
) -> Json<serde_json::Value> {
    let user_email = req.email.trim().to_lowercase();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }
    let user = match sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
        .bind(&user_email)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None)
    {
        Some(u) => u,
        None => return Json(serde_json::json!({"status": "error", "message": "User not found"})),
    };

    if req.is_minor && !req.guardian_consent_given.unwrap_or(false) {
        return Json(
            serde_json::json!({"status": "error", "message": "Guardian consent required for minors"}),
        );
    }

    let _ = sqlx::query(
        "INSERT INTO user_age_verification (user_id, is_minor, guardian_consent_given, guardian_email, age_verified_at, created_at) VALUES (?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) ON CONFLICT(user_id) DO UPDATE SET is_minor = ?, guardian_consent_given = ?, guardian_email = ?, age_verified_at = CURRENT_TIMESTAMP"
    )
    .bind(&user.id)
    .bind(req.is_minor)
    .bind(req.guardian_consent_given.unwrap_or(false))
    .bind(req.guardian_email.clone().unwrap_or_default())
    .bind(req.is_minor)
    .bind(req.guardian_consent_given.unwrap_or(false))
    .bind(req.guardian_email.unwrap_or_default())
    .execute(&state.pool)
    .await;

    Json(serde_json::json!({"status": "success", "message": "Age verification updated"}))
}

async fn get_age_verification(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let user_email = params.get("email").cloned().unwrap_or_default();
    if user_email.is_empty() {
        return Json(serde_json::json!({"status": "error", "message": "Email required"}));
    }

    let verification = sqlx::query_as::<_, UserAgeVerification>(
        "SELECT user_id, is_minor, guardian_consent_given, guardian_email, age_verified_at, created_at FROM user_age_verification WHERE user_id = (SELECT id FROM users WHERE email = ?)"
    )
    .bind(&user_email)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    Json(serde_json::json!({"status": "success", "verification": verification}))
}

async fn post_retention_policy(
    State(state): State<AppState>,
    Json(req): Json<DataRetentionPolicy>,
) -> Json<serde_json::Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let _ = sqlx::query(
        "INSERT INTO data_retention_policies (id, data_type, retention_days, is_active) VALUES (?, ?, ?, ?) ON CONFLICT(data_type) DO UPDATE SET retention_days = ?, is_active = ?, updated_at = CURRENT_TIMESTAMP"
    )
    .bind(&id)
    .bind(&req.data_type)
    .bind(req.retention_days)
    .bind(req.is_active)
    .bind(req.retention_days)
    .bind(req.is_active)
    .execute(&state.pool)
    .await;

    Json(serde_json::json!({"status": "success", "message": "Retention policy updated"}))
}

async fn get_retention_policies(State(state): State<AppState>) -> Json<serde_json::Value> {
    let policies = sqlx::query_as::<_, DataRetentionPolicy>(
        "SELECT id, data_type, retention_days, is_active, updated_at FROM data_retention_policies",
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    Json(serde_json::json!({"status": "success", "policies": policies}))
}

async fn run_data_retention_purge(State(state): State<AppState>) -> Json<serde_json::Value> {
    let policies = sqlx::query_as::<_, DataRetentionPolicy>(
        "SELECT id, data_type, retention_days, is_active, updated_at FROM data_retention_policies WHERE is_active = 1"
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();

    let mut purged = Vec::new();
    for policy in policies {
        let cutoff = format!("datetime('now', '-{} days')", policy.retention_days);

        let affected = match policy.data_type.as_str() {
            "chat_transcripts" => sqlx::query(&format!(
                "DELETE FROM chat_transcripts WHERE created_at < {}",
                cutoff
            ))
            .execute(&state.pool)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0),
            "chat_feedback" => sqlx::query(&format!(
                "DELETE FROM chat_feedback WHERE created_at < {}",
                cutoff
            ))
            .execute(&state.pool)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0),
            "mail_errors" => sqlx::query(&format!(
                "DELETE FROM mail_errors WHERE occurred_at < {}",
                cutoff
            ))
            .execute(&state.pool)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0),
            _ => 0,
        };

        if affected > 0 {
            purged.push(policy.data_type.clone());
        }
    }

    Json(serde_json::json!({"status": "success", "purged": purged}))
}

// ─── Feature Wishes & Voting ────────────────────────────────────────────────

#[derive(sqlx::FromRow)]
struct WishRow {
    id: String,
    title: String,
    description: Option<String>,
    created_by: Option<String>,
    is_official: bool,
    created_at: String,
    vote_count: i64,
}

#[derive(Deserialize)]
struct WishesQuery {
    email: Option<String>,
}

async fn get_wishes(
    State(state): State<AppState>,
    Query(params): Query<WishesQuery>,
) -> impl IntoResponse {
    use crate::models::FeatureWish;

    // Resolve the requesting user's ID (if logged in) so we can compute user_has_voted.
    let user_id_opt: Option<String> = if let Some(ref email) = params.email {
        sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let rows = sqlx::query_as::<_, WishRow>(
        r#"
        SELECT
            w.id         AS id,
            w.title      AS title,
            w.description AS description,
            w.created_by  AS created_by,
            w.is_official AS is_official,
            w.created_at  AS created_at,
            COUNT(v.id)   AS vote_count
        FROM feature_wishes w
        LEFT JOIN feature_votes v ON v.wish_id = w.id
        GROUP BY w.id
        ORDER BY w.is_official DESC, vote_count DESC, w.created_at ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await;

    match rows {
        Ok(rows) => {
            // Fetch voted wish IDs for the logged-in user.
            let voted_ids: Vec<String> = if let Some(ref uid) = user_id_opt {
                sqlx::query_scalar::<_, String>(
                    "SELECT wish_id FROM feature_votes WHERE user_id = ?",
                )
                .bind(uid)
                .fetch_all(&state.pool)
                .await
                .unwrap_or_default()
            } else {
                vec![]
            };

            let wishes: Vec<FeatureWish> = rows
                .into_iter()
                .map(|r| FeatureWish {
                    user_has_voted: voted_ids.contains(&r.id),
                    id: r.id,
                    title: r.title,
                    description: r.description,
                    created_by: r.created_by,
                    is_official: r.is_official,
                    created_at: r.created_at,
                    vote_count: r.vote_count,
                })
                .collect();

            (StatusCode::OK, Json(wishes)).into_response()
        }
        Err(e) => {
            tracing::error!("get_wishes DB error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db error"})),
            )
                .into_response()
        }
    }
}

async fn post_create_wish(
    State(state): State<AppState>,
    Json(body): Json<crate::models::CreateWishRequest>,
) -> impl IntoResponse {
    // Validate inputs to prevent injection / abuse.
    let title = body.title.trim().to_string();
    if title.is_empty() || title.len() > 200 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "title must be 1–200 characters"})),
        )
            .into_response();
    }
    let description = body
        .description
        .as_deref()
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty());

    // Verify the user exists.
    let user_id: Option<String> =
        sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE email = ?")
            .bind(&body.email)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or(None);

    let Some(user_id) = user_id else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "user not found"})),
        )
            .into_response();
    };

    let id = uuid::Uuid::new_v4().to_string();
    let result = sqlx::query(
        "INSERT INTO feature_wishes (id, title, description, created_by, is_official) VALUES (?, ?, ?, ?, 0)"
    )
    .bind(&id)
    .bind(&title)
    .bind(&description)
    .bind(&user_id)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(serde_json::json!({"id": id, "status": "created"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("post_create_wish DB error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "db error"})),
            )
                .into_response()
        }
    }
}

async fn post_vote_wish(
    State(state): State<AppState>,
    axum::extract::Path(wish_id): axum::extract::Path<String>,
    Json(body): Json<crate::models::VoteWishRequest>,
) -> impl IntoResponse {
    // Verify the user exists.
    let user_id: Option<String> =
        sqlx::query_scalar::<_, String>("SELECT id FROM users WHERE email = ?")
            .bind(&body.email)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or(None);

    let Some(user_id) = user_id else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "user not found"})),
        )
            .into_response();
    };

    // Verify the wish exists.
    let wish_exists: bool =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM feature_wishes WHERE id = ?")
            .bind(&wish_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or(0)
            > 0;

    if !wish_exists {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "wish not found"})),
        )
            .into_response();
    }

    // Toggle: if already voted, remove vote; otherwise add it.
    let already_voted: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM feature_votes WHERE wish_id = ? AND user_id = ?",
    )
    .bind(&wish_id)
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0)
        > 0;

    if already_voted {
        let _ = sqlx::query("DELETE FROM feature_votes WHERE wish_id = ? AND user_id = ?")
            .bind(&wish_id)
            .bind(&user_id)
            .execute(&state.pool)
            .await;
        (StatusCode::OK, Json(serde_json::json!({"voted": false}))).into_response()
    } else {
        let vote_id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO feature_votes (id, wish_id, user_id) VALUES (?, ?, ?)",
        )
        .bind(&vote_id)
        .bind(&wish_id)
        .bind(&user_id)
        .execute(&state.pool)
        .await;
        (StatusCode::OK, Json(serde_json::json!({"voted": true}))).into_response()
    }
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
        .route("/rules/delete", post(post_delete_rule))
        .route("/finance/records", get(get_finance_records))
        .route("/finance/monthly", get(get_finance_monthly))
        .route("/emails/process-manual", post(post_manual_process_emails))
        .route("/auto-replies", get(get_auto_replies))
        .route("/auto-replies/update", post(post_update_auto_reply))
        .route("/auto-replies/send", post(post_send_auto_reply))
        .route("/auto-replies/delete", post(post_delete_auto_reply))
        .route("/dashboard", get(get_dashboard))
        .route("/errors", get(get_user_errors))
        .route("/admin/errors", get(get_admin_errors))
        .route("/errors/retry", post(post_retry_mail_error))
        .route("/training/export", get(get_training_export))
        .route("/feedback", get(get_feedback))
        .route("/feedback/mark-read", post(post_mark_feedback_read))
        .route("/feedback/reply", post(post_reply_feedback))
        .route("/chat", post(post_chat))
        .route("/chat/feedback", post(post_chat_feedback))
        .route("/consent/update", post(post_consent_update))
        .route("/consent/history", get(get_consent_history))
        .route("/dsar/request", post(post_dsar_request))
        .route("/dsar/status", get(get_dsar_status))
        .route("/privacy/settings", post(post_privacy_settings))
        .route("/privacy/settings", get(get_privacy_settings))
        .route("/privacy/age-verification", post(post_age_verification))
        .route("/privacy/age-verification", get(get_age_verification))
        .route("/admin/retention/policies", post(post_retention_policy))
        .route("/admin/retention/policies", get(get_retention_policies))
        .route("/admin/retention/purge", post(run_data_retention_purge))
        .route("/wishes", get(get_wishes))
        .route("/wishes", post(post_create_wish))
        .route("/wishes/:id/vote", post(post_vote_wish))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            readonly_write_guard,
        ));

    let app = Router::new()
        .nest("/api", api_router)
        .fallback_service(
            ServeDir::new("frontend/dist")
                .not_found_service(ServeFile::new("frontend/dist/index.html")),
        )
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Web server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
