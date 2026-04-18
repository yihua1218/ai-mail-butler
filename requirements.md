# AI Mail Butler - Requirements Specification

## 1. Project Overview
**Project Name**: AI Mail Butler  
**Version**: 1.0 (Draft)  
**Date**: [Insert Date]  

The AI Mail Butler is a self-hosted email processing assistant platform. It consists of a dedicated Mail Server and a Web-based GUI. Users forward selected emails from their own mail servers to the assistant's mailbox on the `mail.yihua.app` domain. The assistant intelligently identifies the forwarding user, onboards new users via email inquiry, performs AI-driven actions (e.g., translation, auto-reply), and provides a transparent web dashboard for monitoring all processed emails and actions.

The system aims to act as a personal email delegate, reducing manual workload while maintaining full user control and privacy.

## 2. System Architecture
- **Mail Server**: Fully managed SMTP/IMAP server registered under the domain `mail.yihua.app`. A dedicated assistant mailbox (e.g., `assistant@mail.yihua.app` or similar) will be created.
- **Web GUI**: Secure web application providing user dashboard and management interface.
- **Core Components**:
  - Email forwarding detection & parsing engine
  - AI processing module (translation, rule-based auto-reply, etc.)
  - User configuration & onboarding engine (via email)
  - Authentication system using email magic links
  - Logging & audit trail for all received and processed emails

## 3. Functional Requirements

### 3.1 Mail Server & Domain Setup
- Register and maintain the mail domain `mail.yihua.app`.
- Provision a system mailbox for the AI assistant (e.g., `assistant@mail.yihua.app`).
- Support receiving forwarded emails from external user mail servers via a built-in lightweight Rust SMTP server.
- Support sending outbound emails (onboarding inquiries, translated emails, auto-replies, magic login links).

### 3.2 User Registration & Email Forwarding
- Users configure their own mail servers to forward specific email types (by sender, subject, keywords, etc.) to the assistant's mailbox.
- Upon receiving a forwarded email, the system shall:
  - Identify the original forwarding user by inspecting email headers (e.g., `Received`, `X-Forwarded-For`, or custom headers).
  - Check whether the user has an existing configuration profile.

### 3.3 Intelligent Onboarding & Configuration
- If the forwarding user has no prior configuration:
  - The assistant shall automatically send an onboarding email inquiring what actions the user wants the assistant to perform.
- Supported actions include (but are not limited to):
  - Translate incoming English emails into Traditional Chinese (繁體中文) and forward the translated version back to the user.
  - Set up scheduled or rule-based automatic replies (time-based, keyword-based, or sender-based).
  - Any future extensible AI actions (summarization, categorization, etc.).

### 3.4 Email Processing Workflow
- Parse and store every received forwarded email.
- Apply the user’s configured rules or default onboarding flow.
- Execute the requested action (translation, auto-reply, etc.).
- Forward processed results back to the original user when required.
- Log all steps for audit purposes.

### 3.5 Web GUI Features
- **Design & UI**: Apple-style aesthetic using Ant Design components.
- **AI Chat Interface**: An interactive chat interface on the Web UI allowing users to converse directly with the email assistant.
- **Authentication**: Passwordless login using a one-time magic link sent by the AI assistant via email.
- **Dashboard**:
  - View a complete history of all emails received by the assistant on the user’s behalf.
  - Display detailed processing actions taken for each email (e.g., “Translated EN → ZH-TW”, “Sent auto-reply at 09:00”, “Forwarded to user”).
  - Provide configuration interface to manage forwarding rules, action preferences, and schedules.
- Responsive design supporting desktop and mobile access.

## 4. Non-Functional Requirements
- **Privacy & Security**: All emails and user data remain under the user’s control. No third-party cloud email services unless explicitly chosen. Magic-link authentication must be time-limited and single-use.
- **Reliability**: Mail Server must achieve high deliverability and uptime.
- **Scalability**: Design shall support multiple concurrent users and high email volume.
- **Extensibility**: Modular architecture allowing easy addition of new AI actions.
- **Performance**: Email processing latency should be under 30 seconds for typical tasks (translation, simple replies).
- **Technology Stack** (suggested, not mandatory): Open-source mail server (e.g., Postfix + Dovecot), modern web framework (Next.js / Django / FastAPI), AI capabilities via local or API-based LLMs.

## 5. Out of Scope (Phase 1)
- Direct sending/receiving of non-forwarded emails by end users.
- Full email client replacement.
- Multi-domain support (single domain `mail.yihua.app` only in initial version).

## 6. Acceptance Criteria
- Successful end-to-end flow: user forwards email → assistant identifies user → onboarding or processing → action executed → visible in web dashboard.
- Magic-link login works without password.
- Translation accuracy and auto-reply functionality meet user-configured rules.
- All logs and email histories are auditable via the web GUI.

## 7. Appendices
- Domain: `mail.yihua.app`
- Assistant mailbox example: `assistant@mail.yihua.app`
- Target language for translation: Traditional Chinese (繁體中文)
