# AI Mail Butler

AI Mail Butler is a self-hosted email processing assistant platform. It consists of a dedicated Mail Server and a Web-based GUI. Users forward selected emails from their own mail servers to the assistant's mailbox. The assistant intelligently identifies the forwarding user, onboards new users via email inquiry, performs AI-driven actions (e.g., translation, auto-reply), and provides a transparent web dashboard for monitoring all processed emails and actions.

## About the Author and Project Intent

For background on the creator, project motivation, and open collaboration notes:

- [About (English)](About.md)
- [About (Traditional Chinese)](About.zh-TW.md)

## System Architecture

- **Backend**: Rust (`tokio`, `axum`)
- **Database**: SQLite (via `sqlx`)
- **AI Integration**: Custom LLM endpoints via HTTP (`reqwest`)
- **Mail Handling**: SMTP receiving (Samotop) and sending (mail-send) with MIME/HTML support
- **Reliability**: Automated startup diagnostics and database self-healing mechanisms

## Features

- **Email forwarding detection & parsing**: Identifies the original user who forwarded the email.
- **AI processing module**: English to Traditional Chinese translation, rule-based auto-reply.
- **AI Chat Interface**: Converse directly with your email assistant via the Web UI. Anonymous visitors can also chat with the AI to learn about the system. Features detailed chat stats (token count and generation speed).
- **Document-Grounded Answers**: The chat assistant can retrieve relevant content from project documentation and answer with context from matched docs.
- **Documentation Cache Index**: Documentation lookup uses an in-memory cache index with periodic refresh, reducing repeated disk scans while keeping answers current.
- **Chat-Driven Rule Creation**: When users ask in chat to add processing behavior, the assistant can extract and create new email rules automatically (with dedup protection).
- **Long-term Memory**: The AI assistant maintains memory of past interactions to provide more personalized and context-aware responses.
- **Interactive Onboarding**: New users are guided through a series of onboarding questions to set up their preferences and assistant identity.
- **Custom AI Persona**: Users can define their assistant's Chinese and English names, as well as its reply tone (e.g., professional, friendly).
- **Behavioral Analytics**: Automatically tracks user activity and repetitive questions to provide insights for assistant optimization.
- **User configuration & onboarding**: Automated onboarding flow via email.
- **Role-Based Access Control (RBAC)**: Supports Admin, User, and Anonymous roles with strict data isolation.
- **Passwordless Authentication**: Magic link login via email (supports Plain Text & HTML Rich Text, bilingual).
- **User Settings**: Toggle Auto-reply and Dry Run mode (AI replies sent to yourself for review before going to external senders).
- **Web Dashboard**: View history of received emails and actions taken, featuring an Apple-style aesthetic inspired by Ant Design.
- **Built-in SMTP**: Lightweight Rust-native SMTP server for easy deployment.
- **Diagnostics**: Self-healing database schema verification on startup.
- **Readonly Overlay Mode**: Run the application against a data snapshot without modifying it. All writes go to a separate overlay directory; reads transparently fall back to the base snapshot. Useful for demo, staging, and read-only mirror deployments.

## Development and Testing

### Prerequisites
- Rust (cargo)

### Build
```bash
cargo build
```

### Run
```bash
cargo run
```

### CLI Debug Mode (No Backend Server)
Use CLI mode to process local `.eml` files from the spool directory without starting SMTP/Web servers.

Single pass (default):
```bash
cargo run -- --mode cli
```

Custom spool directory + JSON report output:
```bash
cargo run -- --mode cli --spool-dir data/mail_spool --report-json data/mail_spool/cli-report.json
```

Keep source `.eml` files in place for repeated debugging:
```bash
cargo run -- --mode cli --keep-files
```

Watch mode (continuous processing):
```bash
cargo run -- --mode cli --watch
```

Interactive REPL mode:
```bash
cargo run -- --mode cli --repl
```
REPL commands: `list`, `show <index|path>`, `process <index|path>`, `retry-unknown`, `report`, `exit`.

Process one specific `.eml` file:
```bash
cargo run -- --mode cli --eml-file /absolute/path/to/mail_123.eml --keep-files
```

Simulate AI agent steps (rules + memory) and print step-by-step status:
```bash
cargo run -- --mode cli \
    --eml-file /absolute/path/to/mail_123.eml \
    --simulate-agent --simulate-rules --simulate-memory \
    --as-user user@example.com \
    --step --keep-files
```

Notes:
- `--simulate-agent` enables simulation flow.
- `--simulate-rules` checks enabled `email_rules` and generates a simulated auto-reply preview.
- `--simulate-memory` loads `user_memories` and generates a memory-aware simulated reply preview.
- `--as-user` forces a user context for debugging when sender mapping fails.
- `--step` prints each processing stage and simulation progress in CLI output.

Optional docs retrieval controls:
- `DOCS_WHITELIST`: Comma-separated file names or keywords to allow for AI document references. Example: `DOCS_WHITELIST=GMAIL-SMTP-SETUP.md,zh-TW`
- Language preference effect: logged-in users with `preferred_language=zh-TW` will prioritize matches from `*.zh-TW.md` documents.

### Test
```bash
cargo test
```

### Docker Deployment
Build and run in background using Docker Compose:
```bash
docker-compose up --build -d
```

Detailed Deployment Guides (including AWS EC2 implementation):
- [Docker Deployment Guide (English)](DOCKER_AWS_GUIDE.md)
- [Docker 部署指南 (繁體中文)](DOCKER_AWS_GUIDE.zh-TW.md)

## Cloudflare DNS Configuration (MX Records)

To receive emails at your custom domain using AI Mail Butler, you need to configure your DNS settings. Here is an example of configuring MX records in Cloudflare for `mail.example.com`:

1. Log in to your Cloudflare dashboard and select your domain (`example.com`).
2. Navigate to the **DNS** -> **Records** section.
3. First, ensure you have an `A` record pointing to your server's IP address:
   - **Type**: `A`
   - **Name**: `mail` (or your preferred subdomain)
   - **IPv4 address**: `YOUR_SERVER_IP`
   - **Proxy status**: DNS only (Turn OFF the orange cloud, as Cloudflare proxy only supports HTTP/HTTPS, not SMTP).
4. Add the `MX` record to direct emails to your server:
   - **Type**: `MX`
   - **Name**: `mail` (This means you will receive emails at `*@mail.example.com`)
   - **Mail server**: `mail.example.com`
   - **Priority**: `10`

Once DNS propagates, any email sent to `anything@mail.example.com` will be routed to your AI Mail Butler instance.

## SMTP Configuration (Gmail / M365)

To send emails (for Magic Links and AI replies), you need to configure an SMTP relay.

### Gmail
- [Gmail SMTP Setup Guide](docs/GMAIL-SMTP-SETUP.md)
- [Gmail Filter + Forwarding Setup Guide](docs/GMAIL-FILTER-FORWARDING.md)
- [Gmail Filter + Forwarding Setup Guide (Traditional Chinese)](docs/GMAIL-FILTER-FORWARDING.zh-TW.md)
- [Google Workspace Admin Forwarding Allowlist + Policy Guide](docs/GOOGLE-WORKSPACE-FORWARDING-POLICY.md)
- [Google Workspace Admin Forwarding Allowlist + Policy Guide (Traditional Chinese)](docs/GOOGLE-WORKSPACE-FORWARDING-POLICY.zh-TW.md)

### Microsoft 365 (M365)
If you use a Microsoft 365 (M365) account to send emails, you have two options:
1.  **App Passwords (SMTP AUTH)**: Easier for small-scale/personal use.
    - [M365 SMTP Setup Guide](docs/SMTP-SETUP.md)
2.  **OAuth 2.0 (Microsoft Graph)**: Recommended for production (required if SMTP AUTH is disabled by your organization).
    - [English OAuth Setup Guide](docs/M365_OAUTH_SETUP.md)
    - [繁體中文 OAuth 設定指南](docs/M365_OAUTH_SETUP.zh-TW.md)

## Readonly Overlay Mode

Run AI Mail Butler against an existing data snapshot without modifying it. All writes are redirected to an overlay directory; spool and file reads transparently fall back to the base snapshot when the overlay doesn't have the file yet. The API is locked to read-only operations for the duration of the session.

- [Readonly Overlay Mode Guide (English)](docs/READONLY-OVERLAY-MODE.md)
- [唯讀疊加模式說明 (繁體中文)](docs/READONLY-OVERLAY-MODE.zh-TW.md)

## Role-Based Access Control (RBAC)

The system supports strict role-based data isolation. For details on configuring an administrator and the permissions for each role, please read the [RBAC Documentation](docs/RBAC.md).

## How It Works — Feature Guide

For a full explanation of the email forwarding workflow, currently supported AI processing capabilities, and the community Feature Wish Wall (vote on what gets built next), see:

- [How AI Mail Butler Works (English)](docs/HOW-IT-WORKS.md)
- [AI Mail Butler 運作說明 (繁體中文)](docs/HOW-IT-WORKS.zh-TW.md)

## License

This software is released under The Unlicense or CC0 1.0 Universal. See the `LICENSE` file for details.
