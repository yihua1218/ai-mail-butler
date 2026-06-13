use crate::ai::AiClient;
use crate::models::User;
use anyhow::Result;

pub struct OnboardingService;

impl OnboardingService {
    pub async fn extract_preferences(
        client: &AiClient,
        user: &User,
        current_message: &str,
    ) -> Result<String> {
        let system_prompt = "You are an AI assistant that extracts user preferences from a conversation. The user is telling you how they want their emails handled. Output ONLY a concise summary of their preferences, updating any existing preferences. Do not output conversational text.";
        let combined_message = format!(
            "Existing preferences: {}\nUser's new message: {}",
            user.preferences.as_deref().unwrap_or("None"),
            current_message
        );

        let res = client.chat(system_prompt, &combined_message).await?;
        Ok(res.content)
    }

    pub async fn generate_reply(
        client: &AiClient,
        user: &User,
        current_message: &str,
        memory: &str,
        assistant_email: &str,
        pdf_context: Option<String>,
        docs_context: Option<String>,
    ) -> Result<crate::ai::ChatResult> {
        let name_context = if let Some(name) = &user.display_name {
            format!(
                "The user's name is {}. Address them by their name when appropriate.\n",
                name
            )
        } else {
            "".to_string()
        };

        // Custom AI Identity Context
        let ai_name_zh = user.assistant_name_zh.as_deref().unwrap_or("AI 郵件管家");
        let ai_name_en = user
            .assistant_name_en
            .as_deref()
            .unwrap_or("AI Mail Butler");
        let ai_tone_zh = user.assistant_tone_zh.as_deref().unwrap_or("專業且親切");
        let ai_tone_en = user
            .assistant_tone_en
            .as_deref()
            .unwrap_or("professional and friendly");

        let identity_context = format!(
            "Your identity: In Chinese, your name is '{}' and your tone should be '{}'. In English, your name is '{}' and your tone should be '{}'.\n",
            ai_name_zh, ai_tone_zh, ai_name_en, ai_tone_en
        );

        let scope_guard = "SCOPE RULES:\n- You are ONLY an email-assistant for forwarded-email handling, inbox workflow, reply drafting/sending preferences, email rule configuration, dashboard/log interpretation, and related troubleshooting.\n- You MAY do brief small talk/chit-chat, but keep it short and then gently guide back to email-assistant topics.\n- You MUST refuse requests unrelated to email-assistant operations (for example: coding help, writing programs/scripts, general knowledge Q&A not tied to email workflow, math homework, legal/medical analysis outside email-processing context).\n- If refusing, politely state the scope limit in the user's language and offer email-assistant alternatives.\n";

        let system_prompt = if !user.is_onboarded {
            format!("{}{}{}You are an AI Mail Butler. You monitor the email address: {}. The user has just onboarded. Welcome them, and explain that they should forward emails they want you to process to {}. Acknowledge their preferences and ask if there's anything else they need help with.\nIMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context, identity_context, scope_guard, assistant_email, assistant_email)
        } else {
            format!("{}{}{}You are an AI Mail Butler monitoring {}. Acknowledge the user's message based on their known preferences, and ask how you can assist them today. If they ask where to send emails, tell them to forward to {}.\nIMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context, identity_context, scope_guard, assistant_email, assistant_email)
        };

        let memory_instruction = "IMPORTANT: You have access to the user's 'Long-term memory context' below. If the user's current question relates to past conversations, facts you learned before, or previous requests, ALWAYS check the memory context first to provide a consistent and helpful answer.";

        let attachment_context = if let Some(pdf) = pdf_context {
            format!("\n[ATTACHMENT CONTENT (PDF)]:\n{}\n", pdf)
        } else {
            "".to_string()
        };

        let docs_reference_context = if let Some(docs) = docs_context {
            format!("\n[DOCUMENTATION CONTEXT]\n{}\n", docs)
        } else {
            "".to_string()
        };

        let prompt_with_context = format!(
            "{}\n{}\nUser preferences context: {}\nLong-term memory context: {}\n{}\n{}",
            system_prompt,
            memory_instruction,
            user.preferences.as_deref().unwrap_or("None"),
            memory,
            attachment_context,
            docs_reference_context
        );
        client.chat(&prompt_with_context, current_message).await
    }

    pub fn extract_pdf_text(data: &[u8], passwords: &[String]) -> Result<String> {
        use lopdf::Document;
        let mut doc = match Document::load_mem(data) {
            Ok(d) => d,
            Err(e) => return Err(anyhow::anyhow!("Failed to load PDF: {}", e)),
        };

        if doc.is_encrypted() {
            let mut success = false;
            for pwd in passwords {
                if doc.decrypt(pwd.as_bytes()).is_ok() {
                    success = true;
                    break;
                }
            }
            if !success {
                return Err(anyhow::anyhow!(
                    "PDF is encrypted and no valid password was provided."
                ));
            }
        }

        let mut text = String::new();
        let pages = doc.get_pages();
        for (page_num, _) in pages.iter() {
            if let Ok(page_text) = doc.extract_text(&[*page_num]) {
                text.push_str(&page_text);
                text.push_str("\n");
            }
        }
        Ok(text)
    }

    pub async fn get_memory(pool: &sqlx::SqlitePool, user_id: &str) -> String {
        sqlx::query_scalar("SELECT content FROM user_memories WHERE user_id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "No previous memories of this user.".to_string())
    }

    pub async fn update_memory(
        client: &AiClient,
        pool: &sqlx::SqlitePool,
        user_id: &str,
        last_msg: &str,
        last_reply: &str,
    ) -> Result<()> {
        let current_mem = Self::get_memory(pool, user_id).await;
        let system_prompt = "You are an AI assistant that maintains long-term memory of a user. Your goal is to update the 'Memory' based on a new dialogue exchange. Focus on facts, preferences, and important context about the user. Do not include transient details. Output ONLY the updated memory summary.";

        let update_prompt = format!("Current Memory: {}\n\nNew Exchange:\nUser: {}\nAI: {}\n\nPlease provide the updated concise memory summary:", current_mem, last_msg, last_reply);
        let res = client.chat(system_prompt, &update_prompt).await?;

        sqlx::query("INSERT INTO user_memories (id, user_id, content, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP) \
                     ON CONFLICT(id) DO UPDATE SET content = excluded.content, updated_at = CURRENT_TIMESTAMP")
            .bind(user_id) // Using user_id as memory id for 1-to-1 mapping
            .bind(user_id)
            .bind(&res.content)
            .execute(pool).await?;

        Ok(())
    }

    pub async fn get_next_onboarding_question(user: &User) -> Option<&'static str> {
        match user.onboarding_step {
            0 => Some("在開始前，請先確認：你是否同意將你與 AI 助理的對話內容，在脫敏後匯出作為模型訓練資料？（請回答：同意 / 不同意）"),
            1 => Some("接著，請問您主要打算如何使用我？（例如：處理工作郵件、個人生活瑣事，還是整理電子報？）"),
            2 => Some("了解。那麼在回覆郵件時，您希望我展現什麼樣的語氣？（例如：正式專業、輕鬆友善，或是簡短直接？）"),
            3 => Some("沒問題。最後一個基本設定：您希望我在產生回覆後，先寄到您的信箱給您預覽（試運行），還是直接幫您回覆給對方？"),
            _ => None,
        }
    }

    pub async fn log_activity(pool: &sqlx::SqlitePool, user_id: &str, key: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_activity_stats (user_id, activity_key, count, last_occurred) 
             VALUES (?, ?, 1, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, activity_key) DO UPDATE SET 
                count = count + 1,
                last_occurred = CURRENT_TIMESTAMP",
        )
        .bind(user_id)
        .bind(key)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn generate_anonymous_reply(
        client: &AiClient,
        current_message: &str,
        guest_name: Option<String>,
        assistant_email: &str,
        docs_context: Option<String>,
    ) -> Result<crate::ai::ChatResult> {
        let name_context = if let Some(name) = guest_name {
            format!("The person you are talking to is named {}. Address them by their name when appropriate.\n", name)
        } else {
            "".to_string()
        };

        let docs_reference_context = if let Some(docs) = docs_context {
            format!("\n[DOCUMENTATION CONTEXT]\n{}\n", docs)
        } else {
            "".to_string()
        };

        let system_prompt = format!("{}You are AI Mail Butler, an intelligent, self-hosted email processing assistant.
You monitor the following email address for forwarded messages: {}. 
Your capabilities include:
- Auto-replying to forwarded emails based on user instructions.
- A 'Dry Run' mode to let users review drafted responses before sending them to external recipients.
- A Dashboard to view stats and processed emails.
- Role-Based Access Control (Admin vs Regular User).
- Passwordless login using Magic Links.
You are currently talking to an anonymous visitor. Explain these features if asked, and encourage them to forward their emails to {} to see how you can help. Also encourage them to enter their email in the navigation bar to login via Magic Link to use the dashboard and configure you.
    SCOPE RULES:
    - You are ONLY for email-assistant topics related to forwarded-email processing and its settings.
    - You MAY do short casual chit-chat, but keep it brief and guide back to email-assistant tasks.
    - Refuse coding/programming requests and other unrelated requests.
    - When refusing, politely explain your scope and offer email-assistant help.
IMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.
{}", name_context, assistant_email, assistant_email, docs_reference_context);

        client.chat(&system_prompt, current_message).await
    }
}

/// Service for handling email rules, rule matching, and auto-reply generation
pub struct EmailReplyService;

impl EmailReplyService {
    /// Check if an email matches any of the user's enabled rules
    pub async fn find_matching_rule(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        email_subject: &str,
        email_body: &str,
        _email_from: &str,
    ) -> Result<Option<(i64, String, String)>> {
        // Fetch all enabled rules for the user
        let rules: Vec<(i64, String, String)> = sqlx::query_as(
            "SELECT id, rule_text, rule_label FROM email_rules WHERE user_id = ? AND is_enabled = 1"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        // Simple rule matching: check if rule keywords appear in subject or body
        for (rule_id, rule_text, rule_label) in rules {
            let rule_lower = rule_text.to_lowercase();
            let subject_lower = email_subject.to_lowercase();
            let body_lower = email_body.to_lowercase();

            // Extract keywords from rule (simple split by common delimiters)
            let keywords: Vec<&str> = rule_lower
                .split(|c: char| c == ',' || c == ';' || c == '|' || c == '和')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty() && s.len() > 2)
                .collect();

            // Check if any keyword matches in subject or body
            let matched = keywords
                .iter()
                .any(|keyword| subject_lower.contains(keyword) || body_lower.contains(keyword));

            if matched {
                return Ok(Some((rule_id, rule_text, rule_label)));
            }
        }

        Ok(None)
    }

    /// Generate an auto-reply based on a rule
    pub async fn generate_auto_reply(
        client: &AiClient,
        user: &User,
        rule_text: &str,
        original_from: &str,
        original_subject: &str,
        original_body: &str,
    ) -> Result<String> {
        let ai_name_zh = user.assistant_name_zh.as_deref().unwrap_or("AI 郵件管家");
        let ai_name_en = user
            .assistant_name_en
            .as_deref()
            .unwrap_or("AI Mail Butler");
        let ai_tone_zh = user.assistant_tone_zh.as_deref().unwrap_or("專業且親切");
        let ai_tone_en = user
            .assistant_tone_en
            .as_deref()
            .unwrap_or("professional and friendly");

        let identity_context = format!(
            "Your identity: In Chinese, your name is '{}' and your tone should be '{}'. In English, your name is '{}' and your tone should be '{}'.",
            ai_name_zh, ai_tone_zh, ai_name_en, ai_tone_en
        );

        let system_prompt = format!(
            "You are an AI email assistant. {}. Generate a professional email reply based on the given rule/instruction and the original email content. \
             The reply should be concise and appropriate for business communication. \
             Detect the language of the original email and respond in the same language.",
            identity_context
        );

        let user_prompt = format!(
            "Original Email:\nFrom: {}\nSubject: {}\nBody: {}\n\nUser's Rule/Instruction: {}\n\nGenerate a reply following this instruction.",
            original_from, original_subject, original_body, rule_text
        );

        let res = client.chat(&system_prompt, &user_prompt).await?;
        Ok(res.content)
    }

    /// Store an auto-reply in the database
    pub async fn store_auto_reply(
        pool: &sqlx::SqlitePool,
        user_id: &str,
        source_email_id: Option<&str>,
        rule_id: i64,
        original_from: &str,
        original_subject: &str,
        reply_body: &str,
        status: &str,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        sqlx::query(
              "INSERT INTO auto_replies (id, user_id, source_email_id, email_rule_id, original_from, original_subject, reply_body, reply_status, created_at) \
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)"
        )
        .bind(&id)
        .bind(user_id)
           .bind(source_email_id)
        .bind(rule_id)
        .bind(original_from)
        .bind(original_subject)
        .bind(reply_body)
        .bind(status)
        .execute(pool)
        .await?;

        Ok(id)
    }

    /// Get all draft replies for a user (not yet sent)
    pub async fn get_draft_replies(
        pool: &sqlx::SqlitePool,
        user_id: &str,
    ) -> Result<Vec<(String, Option<String>, String, String, String)>> {
        let drafts: Vec<(String, Option<String>, String, String, String)> = sqlx::query_as(
            "SELECT id, source_email_id, original_from, original_subject, reply_body FROM auto_replies WHERE user_id = ? AND reply_status = 'draft' ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(drafts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::User;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn test_config() -> Config {
        Config {
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
            readonly_block_writes: false,
            readonly_base: None,
            overlay_dir: None,
            remote_debug_sshfs_enabled: false,
            remote_debug_mode: "readonly".to_string(),
            remote_debug_access_mode: "readonly".to_string(),
            remote_debug_remote: None,
            remote_debug_mount_point: None,
            remote_debug_overlay_dir: None,
            cloudflare_zone_id: None,
            cloudflare_api_token: None,
            unmatched_rule_guidance_default: false,
        }
    }

    fn test_user() -> User {
        User {
            id: "service-user".to_string(),
            email: "service@example.com".to_string(),
            is_onboarded: true,
            preferences: Some("prefers concise replies".to_string()),
            magic_token: None,
            session_token: None,
            role: "user".to_string(),
            auto_reply: false,
            dry_run: true,
            email_format: "both".to_string(),
            display_name: Some("Service User".to_string()),
            assistant_name_zh: Some("測試管家".to_string()),
            assistant_name_en: Some("Test Butler".to_string()),
            assistant_tone_zh: Some("親切".to_string()),
            assistant_tone_en: Some("warm".to_string()),
            onboarding_step: 4,
            pdf_passwords: None,
            timezone: "UTC".to_string(),
            preferred_language: "en".to_string(),
            training_data_consent: false,
            training_consent_updated_at: None,
            mail_send_method: "dry_run".to_string(),
            rule_label_mode: "local".to_string(),
            time_format: "24h".to_string(),
            date_format: "YYYY-MM-DD".to_string(),
            unmatched_rule_guidance_enabled: None,
        }
    }

    async fn start_mock_ai_server(
        expected_requests: usize,
        content: &'static str,
    ) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock ai");
        let addr = listener.local_addr().expect("mock addr");
        let handle = tokio::spawn(async move {
            for _ in 0..expected_requests {
                let (mut socket, _) = listener.accept().await.expect("accept mock ai");
                let mut buf = Vec::new();
                let mut tmp = [0_u8; 1024];
                loop {
                    let n = socket.read(&mut tmp).await.expect("read request");
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
                        break;
                    }
                }
                let payload = serde_json::json!({
                    "choices": [{"message": {"content": content}, "finish_reason": "stop"}],
                    "usage": {"total_tokens": 9, "completion_tokens": 4, "prompt_tokens": 5}
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
                    .expect("write response");
            }
        });
        (format!("http://{}", addr), handle)
    }

    fn ai_client_with_base_url(base_url: &str) -> AiClient {
        let guard = ENV_LOCK.lock().expect("env lock");
        let old_ai_base = std::env::var("AI_API_BASE_URL").ok();
        std::env::set_var("AI_API_BASE_URL", base_url);
        let client = AiClient::new(&test_config());
        match old_ai_base {
            Some(v) => std::env::set_var("AI_API_BASE_URL", v),
            None => std::env::remove_var("AI_API_BASE_URL"),
        }
        drop(guard);
        client
    }

    #[test]
    fn extract_pdf_text_returns_error_for_invalid_bytes() {
        let bad_pdf = b"this is not a pdf";
        let err = OnboardingService::extract_pdf_text(bad_pdf, &[]).unwrap_err();
        assert!(err.to_string().contains("Failed to load PDF"));
    }

    #[tokio::test]
    async fn onboarding_question_progression_is_correct() {
        let mut user = User {
            id: "u1".to_string(),
            email: "u1@example.com".to_string(),
            is_onboarded: false,
            preferences: None,
            magic_token: None,
            session_token: None,
            role: "user".to_string(),
            auto_reply: false,
            dry_run: true,
            email_format: "both".to_string(),
            display_name: None,
            onboarding_step: 0,
            assistant_name_zh: None,
            assistant_name_en: None,
            assistant_tone_zh: None,
            assistant_tone_en: None,
            mail_send_method: "direct_mx".to_string(),
            pdf_passwords: None,
            timezone: "UTC".to_string(),
            preferred_language: "zh-TW".to_string(),
            training_data_consent: false,
            training_consent_updated_at: None,
            rule_label_mode: "ai_first".to_string(),
            time_format: "24h".to_string(),
            date_format: "auto".to_string(),
            unmatched_rule_guidance_enabled: None,
        };

        assert!(OnboardingService::get_next_onboarding_question(&user)
            .await
            .is_some());
        user.onboarding_step = 1;
        assert!(OnboardingService::get_next_onboarding_question(&user)
            .await
            .is_some());
        user.onboarding_step = 2;
        assert!(OnboardingService::get_next_onboarding_question(&user)
            .await
            .is_some());
        user.onboarding_step = 3;
        assert!(OnboardingService::get_next_onboarding_question(&user)
            .await
            .is_some());
        user.onboarding_step = 4;
        assert!(OnboardingService::get_next_onboarding_question(&user)
            .await
            .is_none());
    }

    #[tokio::test]
    async fn onboarding_ai_helpers_use_mock_client_and_persist_memory() {
        let (base_url, mock_ai) = start_mock_ai_server(5, "mock service reply").await;
        let client = ai_client_with_base_url(&base_url);
        let pool = crate::db::connect("sqlite::memory:")
            .await
            .expect("create schema");
        let user = test_user();
        sqlx::query("INSERT INTO users (id, email, is_onboarded) VALUES (?, ?, 1)")
            .bind(&user.id)
            .bind(&user.email)
            .execute(&pool)
            .await
            .expect("insert user");

        let preferences =
            OnboardingService::extract_preferences(&client, &user, "I like brief replies")
                .await
                .expect("extract preferences");
        assert_eq!(preferences, "mock service reply");

        let reply = OnboardingService::generate_reply(
            &client,
            &user,
            "Where do I forward mail?",
            "Memory says user likes receipts",
            "assistant@example.com",
            Some("PDF says paid".to_string()),
            Some("Docs context".to_string()),
        )
        .await
        .expect("generate reply");
        assert_eq!(reply.content, "mock service reply");
        assert_eq!(reply.total_tokens, 9);

        let anonymous = OnboardingService::generate_anonymous_reply(
            &client,
            "What can you do?",
            Some("Visitor".to_string()),
            "assistant@example.com",
            Some("Docs".to_string()),
        )
        .await
        .expect("anonymous reply");
        assert_eq!(anonymous.content, "mock service reply");

        assert_eq!(
            OnboardingService::get_memory(&pool, &user.id).await,
            "No previous memories of this user."
        );
        OnboardingService::update_memory(&client, &pool, &user.id, "hello", "reply")
            .await
            .expect("update memory");
        assert_eq!(
            OnboardingService::get_memory(&pool, &user.id).await,
            "mock service reply"
        );

        let auto = EmailReplyService::generate_auto_reply(
            &client,
            &user,
            "reply politely",
            "sender@example.com",
            "Question",
            "Please answer",
        )
        .await
        .expect("auto reply");
        assert_eq!(auto, "mock service reply");

        mock_ai.await.expect("mock ai done");
    }

    #[tokio::test]
    async fn rule_matching_activity_and_draft_storage_round_trip() {
        let pool = crate::db::connect("sqlite::memory:")
            .await
            .expect("create schema");
        let user = test_user();
        sqlx::query("INSERT INTO users (id, email, is_onboarded) VALUES (?, ?, 1)")
            .bind(&user.id)
            .bind(&user.email)
            .execute(&pool)
            .await
            .expect("insert user");
        sqlx::query("INSERT INTO email_rules (id, user_id, rule_text, rule_label, is_enabled) VALUES (1, ?, 'invoice, receipt', 'RULE-INVOICE', 1)")
            .bind(&user.id)
            .execute(&pool)
            .await
            .expect("insert rule");

        let matched = EmailReplyService::find_matching_rule(
            &pool,
            &user.id,
            "Monthly invoice",
            "Please see attached",
            "sender@example.com",
        )
        .await
        .expect("find rule")
        .expect("matched rule");
        assert_eq!(matched.0, 1);
        assert_eq!(matched.2, "RULE-INVOICE");

        let no_match = EmailReplyService::find_matching_rule(
            &pool,
            &user.id,
            "Greetings",
            "No keywords",
            "sender@example.com",
        )
        .await
        .expect("find none");
        assert!(no_match.is_none());

        OnboardingService::log_activity(&pool, &user.id, "ask_forwarding_info")
            .await
            .expect("log once");
        OnboardingService::log_activity(&pool, &user.id, "ask_forwarding_info")
            .await
            .expect("log twice");
        let activity_count: i64 = sqlx::query_scalar(
            "SELECT count FROM user_activity_stats WHERE user_id = ? AND activity_key = 'ask_forwarding_info'",
        )
        .bind(&user.id)
        .fetch_one(&pool)
        .await
        .expect("activity count");
        assert_eq!(activity_count, 2);

        let draft_id = EmailReplyService::store_auto_reply(
            &pool,
            &user.id,
            Some("mail-1"),
            1,
            "sender@example.com",
            "Monthly invoice",
            "Draft body",
            "draft",
        )
        .await
        .expect("store draft");
        let drafts = EmailReplyService::get_draft_replies(&pool, &user.id)
            .await
            .expect("drafts");
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].0, draft_id);
        assert_eq!(drafts[0].1.as_deref(), Some("mail-1"));
        assert_eq!(drafts[0].4, "Draft body");
    }
}
