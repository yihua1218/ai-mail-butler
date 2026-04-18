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

    pub async fn generate_reply(client: &AiClient, user: &User, current_message: &str) -> Result<String> {
        let system_prompt = if !user.is_onboarded {
            "You are an AI Mail Butler. The user has just onboarded. Welcome them, acknowledge their preferences, and ask if there's anything else they need help with."
        } else {
            "You are an AI Mail Butler. Acknowledge the user's message based on their known preferences, and ask how you can assist them today."
        };
        
        let prompt_with_context = format!("{} \nUser preferences context: {}", system_prompt, user.preferences.as_deref().unwrap_or("None"));
        let reply = client.chat(&prompt_with_context, current_message).await?;
        Ok(reply)
    }

    pub async fn generate_anonymous_reply(client: &AiClient, current_message: &str) -> Result<String> {
        let system_prompt = "You are AI Mail Butler, an intelligent, self-hosted email processing assistant.
Your capabilities include:
- Auto-replying to forwarded emails based on user instructions.
- A 'Dry Run' mode to let users review drafted responses before sending them to external recipients.
- A Dashboard to view stats and processed emails.
- Role-Based Access Control (Admin vs Regular User).
- Passwordless login using Magic Links.
You are currently talking to an anonymous visitor. Explain these features if asked, and encourage them to enter their email in the navigation bar to login via Magic Link to use the dashboard and configure you.";
        
        let reply = client.chat(system_prompt, current_message).await?;
        Ok(reply)
    }
}
