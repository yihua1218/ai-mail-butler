use anyhow::Result;
use crate::ai::AiClient;
use crate::models::User;

pub struct OnboardingService;

impl OnboardingService {
    pub async fn extract_preferences(client: &AiClient, user: &User, current_message: &str) -> Result<String> {
        let system_prompt = "You are an AI assistant that extracts user preferences from a conversation. The user is telling you how they want their emails handled. Output ONLY a concise summary of their preferences, updating any existing preferences. Do not output conversational text.";
        let combined_message = format!("Existing preferences: {}\nUser's new message: {}", user.preferences.as_deref().unwrap_or("None"), current_message);
        
        let res = client.chat(system_prompt, &combined_message).await?;
        Ok(res.content)
    }

    pub async fn generate_reply(client: &AiClient, user: &User, current_message: &str, memory: &str, assistant_email: &str, pdf_context: Option<String>) -> Result<crate::ai::ChatResult> {
        let name_context = if let Some(name) = &user.display_name {
            format!("The user's name is {}. Address them by their name when appropriate.\n", name)
        } else {
            "".to_string()
        };

        // Custom AI Identity Context
        let ai_name_zh = user.assistant_name_zh.as_deref().unwrap_or("AI 郵件管家");
        let ai_name_en = user.assistant_name_en.as_deref().unwrap_or("AI Mail Butler");
        let ai_tone_zh = user.assistant_tone_zh.as_deref().unwrap_or("專業且親切");
        let ai_tone_en = user.assistant_tone_en.as_deref().unwrap_or("professional and friendly");

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

        let prompt_with_context = format!("{}\n{}\nUser preferences context: {}\nLong-term memory context: {}\n{}", system_prompt, memory_instruction, user.preferences.as_deref().unwrap_or("None"), memory, attachment_context);
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
                return Err(anyhow::anyhow!("PDF is encrypted and no valid password was provided."));
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

    pub async fn update_memory(client: &AiClient, pool: &sqlx::SqlitePool, user_id: &str, last_msg: &str, last_reply: &str) -> Result<()> {
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
            0 => Some("首先，請問您主要打算如何使用我？（例如：處理工作郵件、個人生活瑣事，還是整理電子報？）"),
            1 => Some("了解。那麼在回覆郵件時，您希望我展現什麼樣的語氣？（例如：正式專業、輕鬆友善，或是簡短直接？）"),
            2 => Some("沒問題。最後一個基本設定：您希望我在產生回覆後，先寄到您的信箱給您預覽（試運行），還是直接幫您回覆給對方？"),
            _ => None,
        }
    }

    pub async fn log_activity(pool: &sqlx::SqlitePool, user_id: &str, key: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO user_activity_stats (user_id, activity_key, count, last_occurred) 
             VALUES (?, ?, 1, CURRENT_TIMESTAMP)
             ON CONFLICT(user_id, activity_key) DO UPDATE SET 
                count = count + 1,
                last_occurred = CURRENT_TIMESTAMP"
        )
        .bind(user_id)
        .bind(key)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn generate_anonymous_reply(client: &AiClient, current_message: &str, guest_name: Option<String>, assistant_email: &str) -> Result<crate::ai::ChatResult> {
        let name_context = if let Some(name) = guest_name {
            format!("The person you are talking to is named {}. Address them by their name when appropriate.\n", name)
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
IMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context, assistant_email, assistant_email);
        
        client.chat(&system_prompt, current_message).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::User;

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
            pdf_passwords: None,
            timezone: "UTC".to_string(),
            preferred_language: "zh-TW".to_string(),
        };

        assert!(OnboardingService::get_next_onboarding_question(&user).await.is_some());
        user.onboarding_step = 1;
        assert!(OnboardingService::get_next_onboarding_question(&user).await.is_some());
        user.onboarding_step = 2;
        assert!(OnboardingService::get_next_onboarding_question(&user).await.is_some());
        user.onboarding_step = 3;
        assert!(OnboardingService::get_next_onboarding_question(&user).await.is_none());
    }
}
