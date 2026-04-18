use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use crate::config::Config;

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    base_url: String,
    api_key: String,
    model_name: String,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: String,
}

impl AiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: std::env::var("AI_API_BASE_URL").unwrap_or_else(|_| "http://localhost:1234/v1".to_string()),
            api_key: config.ai_api_key.clone(),
            model_name: std::env::var("AI_MODEL_NAME").unwrap_or_else(|_| "local-model".to_string()),
        }
    }

    pub async fn chat(&self, system_prompt: &str, user_message: &str) -> Result<String> {
        let req_body = ChatRequest {
            model: self.model_name.clone(),
            messages: vec![
                ChatMessage { role: "system".to_string(), content: system_prompt.to_string() },
                ChatMessage { role: "user".to_string(), content: user_message.to_string() },
            ],
        };

        let url = format!("{}/chat/completions", self.base_url);
        let mut req = self.client.post(&url).json(&req_body);
        
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let res = req.send().await?.json::<ChatResponse>().await?;
        
        if let Some(choice) = res.choices.first() {
            Ok(choice.message.content.clone())
        } else {
            Err(anyhow::anyhow!("No choices returned from AI API"))
        }
    }
}
