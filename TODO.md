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
- [ ] Implement consent audit trail table (policy version, consent source, timestamp, optional IP/UA) for legal proof.
- [ ] Add DSAR APIs (access/export/correction/restriction/withdraw-consent) with immutable audit logs.
- [ ] Implement configurable data-retention policy and scheduled purge for `chat_transcripts`, `chat_feedback`, and logs.
- [ ] Add cross-border transfer disclosure fields and data-location reporting for training/export destinations.
- [ ] Add explicit "Do Not Sell/Share" handling and disclosure endpoint for US state privacy expectations.
- [ ] Publish and enforce NY SHIELD-aligned baseline controls (at-rest encryption, access controls, incident response runbook).
- [ ] Add prohibited-use guardrail + policy notice for NYC AEDT-sensitive scenarios (e.g., employment decision support).
- [ ] Add minor-data handling controls (age gate + guardian consent policy hooks) for COPPA-like risk reduction.

## Low Priority
- [ ] Support switching between multiple AI models via UI.
- [ ] Implement attachment processing (e.g., OCR or summarization).
- [ ] Integrate official Google/Microsoft OAuth 2.0.
- [ ] Add data visualization charts for system analytics.
