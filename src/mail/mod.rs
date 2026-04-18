use anyhow::Result;
use tracing::{info, error};
use samotop::server::TcpServer;
use samotop::mail::Builder;
// use samotop_delivery::dir::Dir; // Not needed
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio::fs;
use crate::models::User;
use crate::ai::AiClient;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct MailService;

impl MailService {
    pub async fn start(pool: SqlitePool, ai_client: AiClient) -> Result<()> {
        let spool_dir = "data/mail_spool";
        fs::create_dir_all(spool_dir).await?;

        info!("Starting lightweight SMTP server on port 2525...");
        
        // Use samotop_delivery's Dir if it is under delivery or use samotop::mail::Dir if exported there.
        // I will use samotop_delivery::delivery::Dir since rustc suggested it.
        let dir = samotop_delivery::delivery::Dir::new(spool_dir.to_string().into()).unwrap();
        let mail = Arc::new(Builder::default().using(dir));
        let svc = samotop::io::smtp::SmtpService::new(mail);
        
        // Use 2525 for dev to avoid sudo requirements, forward via firewall in prod
        let srv = TcpServer::on("0.0.0.0:2525").serve(svc);
        
        // Run samotop in the background
        tokio::spawn(async move {
            if let Err(e) = srv.await {
                error!("SMTP server error: {:?}", e);
            }
        });

        // Start mail processor loop
        tokio::spawn(async move {
            Self::process_spool(pool, ai_client, spool_dir).await;
        });

        Ok(())
    }

    async fn process_spool(pool: SqlitePool, ai_client: AiClient, spool_dir: &str) {
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
                                
                                // Simplified mapping: extract the raw email (very naive)
                                let to_clean = to_addr.replace("<", "").replace(">", "");
                                
                                // Find user by email
                                let user = sqlx::query_as::<_, User>("SELECT id, email, is_onboarded, preferences, magic_token FROM users WHERE email = ?")
                                    .bind(&to_clean)
                                    .fetch_optional(&pool).await.unwrap_or(None);

                                if let Some(u) = user {
                                    let id = Uuid::new_v4().to_string();
                                    let preview = body.chars().take(100).collect::<String>();
                                    
                                    sqlx::query("INSERT INTO emails (id, user_id, subject, preview, status) VALUES (?, ?, ?, ?, 'received')")
                                        .bind(&id).bind(&u.id).bind(&subject).bind(&preview)
                                        .execute(&pool).await.unwrap_or_default();

                                    // Trigger AI Auto-reply if they are onboarded
                                    if u.is_onboarded {
                                        info!("Triggering AI for email {}", id);
                                        match crate::services::OnboardingService::generate_reply(&ai_client, &u, &body).await {
                                            Ok(ai_reply) => {
                                                info!("--- AI AUTO REPLY MOCK ---");
                                                info!("To: {}", from_addr);
                                                info!("Subject: Re: {}", subject);
                                                info!("Body:\n{}", ai_reply);
                                                info!("--------------------------");
                                                
                                                sqlx::query("UPDATE emails SET status = 'replied' WHERE id = ?")
                                                    .bind(&id)
                                                    .execute(&pool).await.unwrap_or_default();
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
