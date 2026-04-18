use axum::{
    extract::{State, Query},
    routing::{get, post},
    Json, Router,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::cors::CorsLayer;
use std::net::SocketAddr;
use tracing::info;
use anyhow::Result;
use sqlx::SqlitePool;
use serde::{Deserialize, Serialize};

use crate::models::{User, EmailRecord};
use crate::ai::AiClient;
use crate::services::OnboardingService;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub ai_client: AiClient,
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
        let user = sqlx::query_as::<_, User>("SELECT id, email, is_onboarded, preferences FROM users WHERE email = ?")
            .bind(&email)
            .fetch_optional(&state.pool).await.unwrap_or(None);
        if let Some(user) = user { return Json(Some(user)); }

        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
            .bind(&new_id)
            .bind(&email)
            .execute(&state.pool).await.unwrap();
        return Json(Some(User { id: new_id, email, is_onboarded: false, preferences: None, magic_token: None }));
    }
    Json(None)
}

#[derive(Serialize)]
struct DashboardStats {
    registered_users: i64,
    emails_received: i64,
    emails_replied: i64,
    emails_sent: i64,
}

async fn get_dashboard(
    State(state): State<AppState>,
    Query(query): Query<AuthQuery>,
) -> Json<serde_json::Value> {
    let pool = &state.pool;
    if let Some(email) = query.email {
        if !email.is_empty() {
            let user_id: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE email = ?")
                .bind(&email)
                .fetch_optional(pool).await.unwrap_or(None);
            if let Some((uid,)) = user_id {
                let records = sqlx::query_as::<_, EmailRecord>("SELECT id, subject, preview, status, CAST(received_at AS TEXT) as received_at FROM emails WHERE user_id = ? ORDER BY received_at DESC")
                    .bind(&uid)
                    .fetch_all(pool).await.unwrap_or(vec![]);
                return Json(serde_json::json!({ "type": "personal", "emails": records }));
            }
        }
    }
    
    // Anonymous stats
    let users_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users").fetch_one(pool).await.unwrap_or(0);
    let received_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails").fetch_one(pool).await.unwrap_or(0);
    let replied_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM emails WHERE status = 'replied'").fetch_one(pool).await.unwrap_or(0);
    
    Json(serde_json::json!({
        "type": "public",
        "stats": DashboardStats {
            registered_users: users_count,
            emails_received: received_count,
            emails_replied: replied_count,
            emails_sent: 0, // Mock
        }
    }))
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    email: String,
}

async fn post_chat(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    let email = payload.email;
    let message = payload.message;
    let pool = &state.pool;

    let user = sqlx::query_as::<_, User>("SELECT id, email, is_onboarded, preferences FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(pool).await.unwrap_or(None);

    if let Some(mut user) = user {
        // AI Integration
        let new_pref = match OnboardingService::extract_preferences(&state.ai_client, &user, &message).await {
            Ok(p) => p,
            Err(e) => {
                tracing::error!("Failed to extract preferences: {}", e);
                user.preferences.clone().unwrap_or_default()
            }
        };

        sqlx::query("UPDATE users SET preferences = ?, is_onboarded = true WHERE id = ?")
            .bind(&new_pref)
            .bind(&user.id)
            .execute(pool).await.unwrap();

        user.preferences = Some(new_pref);
        
        let reply = match OnboardingService::generate_reply(&state.ai_client, &user, &message).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("Failed to generate reply: {}", e);
                "Sorry, I am having trouble connecting to my AI brain right now.".to_string()
            }
        };

        return Json(serde_json::json!({ "reply": reply }));
    }

    Json(serde_json::json!({ "reply": "Please login first." }))
}

#[derive(Serialize)]
struct BuildInfo {
    version: &'static str,
    target: &'static str,
    host: &'static str,
    profile: &'static str,
    git_commit: &'static str,
    build_date: &'static str,
}

async fn get_about() -> Json<BuildInfo> {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        target: env!("BUILD_TARGET"),
        host: env!("BUILD_HOST"),
        profile: env!("BUILD_PROFILE"),
        git_commit: env!("GIT_COMMIT"),
        build_date: env!("BUILD_DATE"),
    })
}

#[derive(Deserialize)]
struct MagicLinkRequest {
    email: String,
}

async fn post_magic_link(
    State(state): State<AppState>,
    Json(payload): Json<MagicLinkRequest>,
) -> Json<serde_json::Value> {
    let token = uuid::Uuid::new_v4().to_string();
    let email = payload.email;
    
    // Upsert user and set token
    let user = sqlx::query_as::<_, User>("SELECT id, email, is_onboarded, preferences, magic_token FROM users WHERE email = ?")
        .bind(&email)
        .fetch_optional(&state.pool).await.unwrap_or(None);
        
    if let Some(u) = user {
        sqlx::query("UPDATE users SET magic_token = ? WHERE id = ?")
            .bind(&token)
            .bind(&u.id)
            .execute(&state.pool).await.unwrap();
    } else {
        let new_id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO users (id, email, magic_token) VALUES (?, ?, ?)")
            .bind(&new_id)
            .bind(&email)
            .bind(&token)
            .execute(&state.pool).await.unwrap();
    }
    
    let login_url = format!("http://localhost:3000/login?token={}", token);
    
    // MOCK SEND EMAIL:
    tracing::info!("--- MOCK EMAIL ---");
    tracing::info!("To: {}", email);
    tracing::info!("Subject: Your AI Mail Butler Login Link");
    tracing::info!("Body: Click here to login: {}", login_url);
    tracing::info!("------------------");
    
    Json(serde_json::json!({ "status": "success", "message": "Magic link sent to console mock" }))
}

#[derive(Deserialize)]
struct VerifyRequest {
    token: String,
}

async fn post_verify(
    State(state): State<AppState>,
    Json(payload): Json<VerifyRequest>,
) -> Json<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT id, email, is_onboarded, preferences, magic_token FROM users WHERE magic_token = ?")
        .bind(&payload.token)
        .fetch_optional(&state.pool).await.unwrap_or(None);
        
    if let Some(mut u) = user {
        // Clear token
        sqlx::query("UPDATE users SET magic_token = NULL WHERE id = ?")
            .bind(&u.id)
            .execute(&state.pool).await.unwrap();
            
        u.magic_token = None;
        Json(Some(u))
    } else {
        Json(None)
    }
}

pub async fn start_server(port: u16, state: AppState) -> Result<()> {
    let api_router = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/about", get(get_about))
        .route("/auth/magic-link", post(post_magic_link))
        .route("/auth/verify", post(post_verify))
        .route("/me", get(get_me))
        .route("/dashboard", get(get_dashboard))
        .route("/chat", post(post_chat));

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
