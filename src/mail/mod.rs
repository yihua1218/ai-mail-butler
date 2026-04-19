use anyhow::Result;
use tracing::{info, error};
use samotop::server::TcpServer;
use samotop::mail::Builder;
use mailparse::MailHeaderMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio::fs;
use crate::models::User;
use crate::ai::AiClient;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::config::Config;


pub struct MailService;

impl MailService {
    pub async fn start(pool: SqlitePool, ai_client: AiClient, config: Arc<Config>) -> Result<()> {
        let spool_dir = "data/mail_spool";
        fs::create_dir_all(spool_dir).await?;

        info!("Starting lightweight SMTP server on port 2525...");
        
        let dir = samotop_delivery::delivery::Dir::new(spool_dir.to_string().into()).unwrap();
        let mail = Arc::new(Builder::default().using(dir));
        let svc = samotop::io::smtp::SmtpService::new(mail);
        
        let srv = TcpServer::on("0.0.0.0:2525").serve(svc);
        
        tokio::spawn(async move {
            if let Err(e) = srv.await {
                error!("SMTP server error: {:?}", e);
            }
        });

        tokio::spawn(async move {
            Self::process_spool(pool, ai_client, config, spool_dir).await;
        });

        Ok(())
    }

    async fn process_spool(pool: SqlitePool, ai_client: AiClient, config: Arc<Config>, spool_dir: &str) {
        info!("Started mail spool processor watching {}", spool_dir);
        loop {
            if let Ok(mut entries) = fs::read_dir(spool_dir).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(contents) = fs::read(&path).await {
                            if let Ok(parsed) = mailparse::parse_mail(&contents) {
                                let subject = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("Subject"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_else(|| "No Subject".to_string());
                                    
                                let to_addr = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("To"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_default();
                                    
                                let from_addr = parsed.headers.iter()
                                    .find(|h| h.get_key().eq_ignore_ascii_case("From"))
                                    .map(|h| h.get_value())
                                    .unwrap_or_default();

                                let body = parsed.get_body().unwrap_or_default();
                                
                                info!("Parsed email from {} to {}: {}", from_addr, to_addr, subject);
                                
                                let to_clean = to_addr.replace("<", "").replace(">", "");
                                
                                let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
                                    .bind(&to_clean)
                                    .fetch_optional(&pool).await.unwrap_or(None);

                                if let Some(u) = user {
                                    let id = Uuid::new_v4().to_string();
                                    let preview = body.chars().take(100).collect::<String>();
                                    
                                    sqlx::query("INSERT INTO emails (id, user_id, subject, preview, status) VALUES (?, ?, ?, ?, 'received')")
                                        .bind(&id).bind(&u.id).bind(&subject).bind(&preview)
                                        .execute(&pool).await.unwrap_or_default();

                                let mut pdf_texts = Vec::new();
                                let passwords: Vec<String> = u.pdf_passwords.as_ref()
                                    .and_then(|s| serde_json::from_str(s).ok())
                                    .unwrap_or_default();

                                for part in parsed.subparts.iter() {
                                    if part.get_headers().get_first_header("Content-Type").map(|h| h.get_value()).unwrap_or_default().contains("application/pdf") {
                                        if let Ok(data) = part.get_body_raw() {
                                            if let Ok(text) = crate::services::OnboardingService::extract_pdf_text(&data, &passwords) {
                                                pdf_texts.push(text);
                                            }
                                        }
                                    }
                                }
                                let pdf_context = if pdf_texts.is_empty() { None } else { Some(pdf_texts.join("\n---\n")) };

                                if u.is_onboarded {
                                    info!("Triggering AI for email {}", id);
                                    let memory = crate::services::OnboardingService::get_memory(&pool, &u.id).await;
                                    match crate::services::OnboardingService::generate_reply(&ai_client, &u, &body, &memory, &config.assistant_email, pdf_context).await {
                                            Ok(res) => {
                                                let ai_reply = res.content;
                                                // Update memory asynchronously
                                                let ai_client_clone = ai_client.clone();
                                                let pool_clone = pool.clone();
                                                let user_id = u.id.clone();
                                                let msg_clone = body.clone();
                                                let reply_clone = ai_reply.clone();
                                                tokio::spawn(async move {
                                                    let _ = crate::services::OnboardingService::update_memory(&ai_client_clone, &pool_clone, &user_id, &msg_clone, &reply_clone).await;
                                                });
                                                // Handle dry run and auto reply logic
                                                let target_email = if u.dry_run {
                                                    u.email.clone() // Send back to the user
                                                } else {
                                                    from_addr.clone() // Send to original sender
                                                };

                                                let _email_subject = format!("Re: {}", subject);
                                                
                                                if u.dry_run || u.auto_reply {
                                                    info!("Sending AI reply to {} (dry_run: {}, auto_reply: {})", target_email, u.dry_run, u.auto_reply);
                                                    
                                                    let host = config.smtp_relay_host.clone().unwrap_or_default();
                                                    let port = config.smtp_relay_port;
                                                    let user = config.smtp_relay_user.clone().unwrap_or_default();
                                                    let pass = config.smtp_relay_pass.clone().unwrap_or_default();
                                                    
                                                    let message = mail_send::mail_builder::MessageBuilder::new()
                                                        .from(config.assistant_email.clone())
                                                        .to(target_email.replace("<", "").replace(">", ""))
                                                        .subject(format!("Re: {}", subject))
                                                        .text_body(ai_reply);

                                                    let is_implicit = port == 465;
                                                    let pool_clone = pool.clone();
                                                    let email_id = id.clone();

                                                    tokio::spawn(async move {
                                                        let mut builder = mail_send::SmtpClientBuilder::new(host.as_str(), port);
                                                        builder = builder.implicit_tls(is_implicit);
                                                        if !user.is_empty() {
                                                            builder = builder.credentials((user.as_str(), pass.as_str()));
                                                        }
                                                        
                                                        match builder.connect().await {
                                                            Ok(mut client) => {
                                                                if let Err(e) = client.send(message).await {
                                                                    error!("Failed to send AI reply via mail-send: {:?}", e);
                                                                } else {
                                                                    sqlx::query("UPDATE emails SET status = 'replied' WHERE id = ?")
                                                                        .bind(&email_id).execute(&pool_clone).await.unwrap_or_default();
                                                                }
                                                            }
                                                            Err(e) => error!("Failed to connect to SMTP for AI reply: {:?}", e),
                                                        }
                                                    });
                                                } else {
                                                    info!("Skipping reply for {} (dry_run: false, auto_reply: false)", id);
                                                    sqlx::query("UPDATE emails SET status = 'drafted' WHERE id = ?")
                                                        .bind(&id).execute(&pool).await.unwrap_or_default();
                                                }
                                            },
                                            Err(e) => error!("AI reply failed: {}", e)
                                        }
                                    }
                                } else {
                                    info!("User not found for {}", to_clean);
                                }
                            }
                        }
                        let _ = fs::remove_file(path).await;
                    }
                }
            }
            sleep(Duration::from_secs(3)).await;
        }
    }
}
