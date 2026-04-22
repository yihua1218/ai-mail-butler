# How AI Mail Butler Works — Feature Guide

This guide introduces the core workflow of AI Mail Butler, explains the currently supported processing capabilities, and describes the **Feature Wish Wall** where registered users can propose and vote on future features.

---

## Overview

AI Mail Butler acts as an intelligent email processing agent. Rather than connecting directly to your existing mailbox, you selectively **forward** specific emails to the AI assistant's dedicated mailbox. The assistant then processes them according to the rules you define.

```
Your Inbox
    │
    │  (email forwarding filter — configured once)
    ▼
AI Assistant Mailbox   ←── receives forwarded mail
    │
    ▼
AI Processing Engine
    ├──▶  Auto Reply         (draft or send replies per your rules)
    ├──▶  Bill Accounting    (extract & aggregate financial data)
    └──▶  More features…     (vote on the Wish Wall below)
```

---

## Getting Started in 3 Steps

### Step 1 — Get Your AI Mailbox Address

After logging in, navigate to the **About** page (`/about`). Your dedicated AI assistant email address is displayed there and can be copied with one click.

Example: `assistant@mail.your-domain.com`

### Step 2 — Configure Forwarding Rules in Your Email Provider

In Gmail, Outlook, or any mail service that supports forwarding filters:

1. Add the AI assistant mailbox as a verified forwarding address.
2. Create a filter (e.g., by sender domain, subject keyword, or label) to auto-forward matching emails to the AI mailbox.

Detailed setup guides:
- [Gmail Filter & Forwarding Setup](GMAIL-FILTER-FORWARDING.md)
- [Gmail Filter & Forwarding Setup (Traditional Chinese)](GMAIL-FILTER-FORWARDING.zh-TW.md)
- [Google Workspace Admin Forwarding Policy](GOOGLE-WORKSPACE-FORWARDING-POLICY.md)

### Step 3 — Define Email Processing Rules

In the **Rules** page (`/rules`) or directly via **AI Chat** (`/chat`), tell the assistant how to handle each category of email. Examples:

- "If the email is a billing notification from vendor.com, archive it to the finance report."
- "If the subject contains 'contract renewal', draft a reply asking for confirmation."

Rules can be created manually or by chatting naturally with the assistant — it will extract actionable rules from your instructions automatically.

---

## Currently Supported Features

### Auto Reply

The AI assistant analyzes the content of each incoming email and generates a reply draft based on your configured rules. You can operate in **Dry Run** mode (replies are sent to yourself first for review) or fully automatic mode (replies are sent directly to the original sender).

**Customizable options:**
- Assistant Chinese / English display name
- Reply tone (professional, friendly, concise, etc.)
- Reply language auto-detection or fixed language

### Bill & Finance Accounting

The assistant automatically extracts financial data from emails such as invoices, bank statements, credit card bills, and payment notifications. Extracted data is aggregated into monthly finance summaries, visible in the **Finance** page (`/finance`).

**Extracted fields include:**
- Transaction amount & currency
- Payment due date
- Statement amount
- Issuing bank & card last 4 digits
- Finance category (income / expense / bill)
- Month key for aggregation

---

## Feature Wish Wall

The **How It Works** page (`/how-it-works`) includes a community **Wish Wall** where registered users can:

1. **Browse** existing feature suggestions (official roadmap items + community proposals).
2. **Vote** for features they want most — one vote per user per feature, toggleable.
3. **Submit** their own feature requests with a title and optional description.

Features with the most votes are prioritized for development.

### API Endpoints

| Method | Path                        | Description                                                                                                             |
| ------ | --------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| `GET`  | `/api/wishes?email=<email>` | List all wishes with vote counts. If `email` is provided and the user is logged in, includes `user_has_voted` per item. |
| `POST` | `/api/wishes`               | Submit a new wish. Body: `{ email, title, description? }`                                                               |
| `POST` | `/api/wishes/:id/vote`      | Toggle vote on a wish. Body: `{ email }` — adds vote if not voted, removes if already voted.                            |

### Seeded Official Wishes

At startup, the following items are automatically seeded (idempotent — safe to restart):

| ID                          | Title                                     | Status   |
| --------------------------- | ----------------------------------------- | -------- |
| `official-auto-reply`       | 自動回信 / Auto Reply                     | ✅ Live   |
| `official-bill-accounting`  | 帳務整理 / Bill & Finance Accounting      | ✅ Live   |
| `wish-smart-labels`         | 智慧分類標籤 / Smart Label Classification | 🗳️ Voting |
| `wish-meeting-summary`      | 會議邀請摘要 / Meeting Invitation Summary | 🗳️ Voting |
| `wish-subscription-tracker` | 訂閱追蹤 / Subscription Tracker           | 🗳️ Voting |

---

## Database Schema

Two tables are added to the SQLite database:

```sql
-- Feature wish list
CREATE TABLE feature_wishes (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    created_by  TEXT,              -- NULL for official/seeded items
    is_official BOOLEAN NOT NULL DEFAULT 0,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- One vote per user per wish (enforced by UNIQUE constraint)
CREATE TABLE feature_votes (
    id         TEXT PRIMARY KEY NOT NULL,
    wish_id    TEXT NOT NULL,
    user_id    TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(wish_id, user_id),
    FOREIGN KEY(wish_id) REFERENCES feature_wishes(id),
    FOREIGN KEY(user_id) REFERENCES users(id)
);
```

Both tables are created idempotently on startup via `CREATE TABLE IF NOT EXISTS` — safe to run on existing databases.

---

## Security Notes

- **Authentication required for write operations**: wish submission and voting both require a valid registered `email` that resolves to an existing user in the database. Unregistered emails are rejected with `401 Unauthorized`.
- **Input validation**: wish titles are capped at 200 characters; empty titles are rejected at both frontend (form validation) and backend (API guard).
- **Vote toggle is idempotent**: submitting a vote twice removes it rather than double-counting — prevents vote inflation.
- **No rate limiting is currently applied** to the wish/vote endpoints. Consider adding it if public-facing abuse is a concern.

---

## Related Documentation

- [Gmail Filter & Forwarding Setup](GMAIL-FILTER-FORWARDING.md)
- [Google Workspace Forwarding Policy](GOOGLE-WORKSPACE-FORWARDING-POLICY.md)
- [M365 OAuth Setup](M365_OAUTH_SETUP.md)
- [RBAC — Role-Based Access Control](RBAC.md)
