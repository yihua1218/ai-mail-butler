use crate::config::Config;
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    usage: Option<UsageInfo>,
}

#[derive(Deserialize)]
struct UsageInfo {
    total_tokens: u32,
    #[allow(dead_code)]
    completion_tokens: u32,
    #[allow(dead_code)]
    prompt_tokens: u32,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

pub struct ChatResult {
    pub content: String,
    pub total_tokens: u32,
    pub duration_ms: u64,
    pub finish_reason: Option<String>,
}

impl AiClient {
    pub fn new(config: &Config) -> Self {
        Self {
            client: Client::new(),
            base_url: std::env::var("AI_API_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:1234/v1".to_string()),
            api_key: config.ai_api_key.clone(),
            model_name: std::env::var("AI_MODEL_NAME")
                .unwrap_or_else(|_| "local-model".to_string()),
        }
    }

    pub async fn chat(&self, system_prompt: &str, user_message: &str) -> Result<ChatResult> {
        let req_body = ChatRequest {
            model: self.model_name.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
        };

        let url = format!("{}/chat/completions", self.base_url);
        let mut req = self.client.post(&url).json(&req_body);

        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }

        let start = std::time::Instant::now();
        let response = req.send().await?;
        let status = response.status();
        let body = response.text().await?;
        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "AI API returned HTTP {}: {}",
                status,
                summarize_body(&body)
            ));
        }

        let res = parse_chat_response(&body).map_err(|e| {
            anyhow::anyhow!(
                "failed to decode AI response body: {}; body={}",
                e,
                summarize_body(&body)
            )
        })?;
        let duration = start.elapsed().as_millis() as u64;

        if let Some(choice) = res.choices.first() {
            let total_tokens = res.usage.map(|u| u.total_tokens).unwrap_or(0);
            Ok(ChatResult {
                content: choice.message.content.clone().unwrap_or_default(),
                total_tokens,
                duration_ms: duration,
                finish_reason: choice.finish_reason.clone(),
            })
        } else {
            Err(anyhow::anyhow!("No choices returned from AI API"))
        }
    }
}

fn summarize_body(body: &str) -> String {
    let normalized = body.split_whitespace().collect::<Vec<_>>().join(" ");
    let summary: String = normalized.chars().take(800).collect();
    if normalized.chars().count() > 800 {
        format!("{summary}...")
    } else {
        summary
    }
}

fn parse_chat_response(body: &str) -> Result<ChatResponse, serde_json::Error> {
    match serde_json::from_str::<ChatResponse>(body) {
        Ok(res) => Ok(res),
        Err(first_err) => {
            if let Some(json) = extract_sse_chat_json(body) {
                serde_json::from_str::<ChatResponse>(&json)
            } else if let Ok(value) = serde_json::from_str::<Value>(body) {
                parse_compatible_chat_response(value).map_err(|_| first_err)
            } else {
                Err(first_err)
            }
        }
    }
}

fn extract_sse_chat_json(body: &str) -> Option<String> {
    for line in body.lines() {
        let line = line.trim();
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if data.starts_with('{') {
            return Some(data.to_string());
        }
    }
    None
}

fn parse_compatible_chat_response(value: Value) -> Result<ChatResponse> {
    if let Some(content) = value.get("content").and_then(Value::as_str) {
        return Ok(ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessageResponse {
                    content: Some(content.to_string()),
                },
                finish_reason: None,
            }],
            usage: None,
        });
    }

    if let Some(response) = value.get("response").and_then(Value::as_str) {
        return Ok(ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessageResponse {
                    content: Some(response.to_string()),
                },
                finish_reason: None,
            }],
            usage: None,
        });
    }

    if let Some(text) = value.get("text").and_then(Value::as_str) {
        return Ok(ChatResponse {
            choices: vec![ChatChoice {
                message: ChatMessageResponse {
                    content: Some(text.to_string()),
                },
                finish_reason: None,
            }],
            usage: None,
        });
    }

    serde_json::from_value(value).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_body_normalizes_whitespace_and_truncates() {
        assert_eq!(
            summarize_body("hello\n\n  world\tagain"),
            "hello world again"
        );

        let long = "x ".repeat(900);
        let summary = summarize_body(&long);
        assert!(summary.ends_with("..."));
        assert!(summary.chars().count() <= 803);
    }

    #[test]
    fn parse_chat_response_handles_openai_shape() {
        let body = serde_json::json!({
            "choices": [{
                "message": { "content": "hello" },
                "finish_reason": "stop"
            }],
            "usage": {
                "total_tokens": 12,
                "completion_tokens": 5,
                "prompt_tokens": 7
            }
        })
        .to_string();

        let parsed = parse_chat_response(&body).expect("parse openai response");
        assert_eq!(parsed.choices[0].message.content.as_deref(), Some("hello"));
        assert_eq!(parsed.choices[0].finish_reason.as_deref(), Some("stop"));
        assert_eq!(parsed.usage.map(|u| u.total_tokens), Some(12));
    }

    #[test]
    fn parse_chat_response_handles_sse_data_line() {
        let body = "event: message\n\
                    data: {\"choices\":[{\"message\":{\"content\":\"from sse\"},\"finish_reason\":\"stop\"}]}\n\
                    data: [DONE]\n";

        let parsed = parse_chat_response(body).expect("parse sse response");
        assert_eq!(
            parsed.choices[0].message.content.as_deref(),
            Some("from sse")
        );
    }

    #[test]
    fn parse_chat_response_handles_compatible_content_response_and_text() {
        for (key, expected) in [
            ("content", "from content"),
            ("response", "from response"),
            ("text", "from text"),
        ] {
            let body = serde_json::json!({ key: expected }).to_string();
            let parsed = parse_chat_response(&body).expect("parse compatible response");
            assert_eq!(parsed.choices[0].message.content.as_deref(), Some(expected));
            assert!(parsed.usage.is_none());
        }
    }

    #[test]
    fn extract_sse_chat_json_skips_done_and_empty_lines() {
        let body = "data:\n\ndata: [DONE]\n\ndata: {\"content\":\"ok\"}\n";
        assert_eq!(
            extract_sse_chat_json(body).as_deref(),
            Some("{\"content\":\"ok\"}")
        );
    }
}
