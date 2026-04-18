# AI Mail Butler

AI Mail Butler is a self-hosted email processing assistant platform. It consists of a dedicated Mail Server and a Web-based GUI. Users forward selected emails from their own mail servers to the assistant's mailbox. The assistant intelligently identifies the forwarding user, onboards new users via email inquiry, performs AI-driven actions (e.g., translation, auto-reply), and provides a transparent web dashboard for monitoring all processed emails and actions.

## System Architecture

- **Backend**: Rust (`tokio`, `axum`)
- **Database**: SQLite (via `sqlx`)
- **AI Integration**: Custom LLM endpoints via HTTP (`reqwest`)
- **Mail Handling**: SMTP receiving and sending (e.g. `lettre`, `mailparse`)

## Features

- **Email forwarding detection & parsing**: Identifies the original user who forwarded the email.
- **AI processing module**: English to Traditional Chinese translation, rule-based auto-reply.
- **AI Chat Interface**: Converse directly with your email assistant via the Web UI.
- **User configuration & onboarding**: Automated onboarding flow via email.
- **Passwordless Authentication**: Magic link login.
- **Web Dashboard**: View history of received emails and actions taken, featuring an Apple-style aesthetic inspired by Ant Design.
- **Built-in SMTP**: Lightweight Rust-native SMTP server for easy deployment.

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

### Test
```bash
cargo test
```

### Docker Deployment
To build and run using Docker:
```bash
docker-compose up --build -d
```

## Cloudflare DNS Configuration (MX Records)

To receive emails at your custom domain using AI Mail Butler, you need to configure your DNS settings. Here is an example of configuring MX records in Cloudflare for `mail.yihua.app`:

1. Log in to your Cloudflare dashboard and select your domain (`yihua.app`).
2. Navigate to the **DNS** -> **Records** section.
3. First, ensure you have an `A` record pointing to your server's IP address:
   - **Type**: `A`
   - **Name**: `mail` (or your preferred subdomain)
   - **IPv4 address**: `YOUR_SERVER_IP`
   - **Proxy status**: DNS only (Turn OFF the orange cloud, as Cloudflare proxy only supports HTTP/HTTPS, not SMTP).
4. Add the `MX` record to direct emails to your server:
   - **Type**: `MX`
   - **Name**: `mail` (This means you will receive emails at `*@mail.yihua.app`)
   - **Mail server**: `mail.yihua.app`
   - **Priority**: `10`

Once DNS propagates, any email sent to `anything@mail.yihua.app` will be routed to your AI Mail Butler instance.

## License

This software is released under The Unlicense or CC0 1.0 Universal. See the `LICENSE` file for details.
