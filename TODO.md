# TODO

## High Priority
- [x] Implement lightweight Rust-native SMTP server in `src/mail/mod.rs` (via `samotop`).
- [x] Configure DB connection and `sqlx` migrations in `src/db/mod.rs`.
- [x] Build Axum Web server and API routes in `src/web/mod.rs`.
- [x] Initialize React frontend with Ant Design (Apple-inspired aesthetics).

## Medium Priority
- [x] Implement AI Chat functionality (REST API).
- [x] Implement email parser for identifying forwarding users and content.
- [x] Develop AI integration module for translation and auto-reply.
- [x] Create bilingual Email Magic-link login (HTML/Plain text support).
- [x] Add automated startup diagnostics and DB self-healing.
- [x] Implement user-configurable email formats.
- [x] Implement Guest Mode persistence (localStorage).
- [x] Implement AI Long-term Memory and Custom Assistant Identity.
- [x] Implement Interactive Onboarding flow.
- [x] Implement Chat Performance Metadata (Tokens/Speed).
- [x] Create AWS EC2 Docker Deployment Guides (EN/ZH).
- [ ] Add unit and integration tests.
- [x] Optimize Docker build process (Frontend dist bundling).

## Low Priority
- [ ] Support switching between multiple AI models via UI.
- [ ] Implement attachment processing (e.g., OCR or summarization).
- [ ] Integrate official Google/Microsoft OAuth 2.0.
- [ ] Add data visualization charts for system analytics.
