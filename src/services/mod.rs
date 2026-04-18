use anyhow::Result;
use crate::ai::AiClient;
use crate::models::User;

pub struct OnboardingService;

impl OnboardingService {
    pub async fn extract_preferences(client: &AiClient, user: &User, current_message: &str) -> Result<String> {
        let system_prompt = "You are an AI assistant that extracts user preferences from a conversation. The user is telling you how they want their emails handled. Output ONLY a concise summary of their preferences, updating any existing preferences. Do not output conversational text.";
        let combined_message = format!("Existing preferences: {}\nUser's new message: {}", user.preferences.as_deref().unwrap_or("None"), current_message);
        
        let new_pref = client.chat(system_prompt, &combined_message).await?;
        Ok(new_pref)
    }

    pub async fn generate_reply(client: &AiClient, user: &User, current_message: &str, memory: &str) -> Result<String> {
        let name_context = if let Some(name) = &user.display_name {
            format!("The user's name is {}. Address them by their name when appropriate.\n", name)
        } else {
            "".to_string()
        };

        let system_prompt = if !user.is_onboarded {
            format!("{}You are an AI Mail Butler. The user has just onboarded. Welcome them, acknowledge their preferences, and ask if there's anything else they need help with.\nIMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context)
        } else {
            format!("{}You are an AI Mail Butler. Acknowledge the user's message based on their known preferences, and ask how you can assist them today.\nIMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context)
        };
        
        let prompt_with_context = format!("{}\nUser preferences context: {}\nLong-term memory context: {}", system_prompt, user.preferences.as_deref().unwrap_or("None"), memory);
        let reply = client.chat(&prompt_with_context, current_message).await?;
        Ok(reply)
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
        let updated_mem = client.chat(system_prompt, &update_prompt).await?;

        sqlx::query("INSERT INTO user_memories (id, user_id, content, updated_at) VALUES (?, ?, ?, CURRENT_TIMESTAMP) \
                     ON CONFLICT(id) DO UPDATE SET content = excluded.content, updated_at = CURRENT_TIMESTAMP")
            .bind(user_id) // Using user_id as memory id for 1-to-1 mapping
            .bind(user_id)
            .bind(&updated_mem)
            .execute(pool).await?;
        
        Ok(())
    }

    pub async fn generate_anonymous_reply(client: &AiClient, current_message: &str, guest_name: Option<String>) -> Result<String> {
        let name_context = if let Some(name) = guest_name {
            format!("The person you are talking to is named {}. Address them by their name when appropriate.\n", name)
        } else {
            "".to_string()
        };

        let system_prompt = format!("{}You are AI Mail Butler, an intelligent, self-hosted email processing assistant.
Your capabilities include:
- Auto-replying to forwarded emails based on user instructions.
- A 'Dry Run' mode to let users review drafted responses before sending them to external recipients.
- A Dashboard to view stats and processed emails.
- Role-Based Access Control (Admin vs Regular User).
- Passwordless login using Magic Links.
You are currently talking to an anonymous visitor. Explain these features if asked, and encourage them to enter their email in the navigation bar to login via Magic Link to use the dashboard and configure you.
IMPORTANT: Detect the language of the user's message. Default to Traditional Chinese (繁體中文) unless the user explicitly writes in Simplified Chinese (簡體中文). If the user writes in English or other languages, respond in that language.", name_context);
        
        let reply = client.chat(&system_prompt, current_message).await?;
        Ok(reply)
    }
}
