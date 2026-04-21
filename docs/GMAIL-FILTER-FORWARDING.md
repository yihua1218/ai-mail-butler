# Gmail Filter and Forwarding Setup for AI Mail Butler

This guide explains how to:
1. Verify your AI assistant mailbox as a forwarding target in Gmail.
2. Create Gmail filters to auto-forward only the emails you want AI Mail Butler to process.

### 1) Prerequisites
- Your AI Mail Butler mailbox is ready (example: `assistant@mail.example.com`).
- You can receive emails at that mailbox.

### 2) Add and verify forwarding address in Gmail
1. Open Gmail and click the gear icon -> **See all settings**.
2. Go to **Forwarding and POP/IMAP**.
3. Click **Add a forwarding address**.
4. Enter your AI assistant mailbox, for example `assistant@mail.example.com`.
5. Gmail sends a verification email to that mailbox.
6. Open the verification mail and click the confirmation link (or enter the code in Gmail).

Important: Gmail will not auto-forward until the forwarding address is verified.

### 3) Create a filter for selective forwarding
1. In Gmail search bar, click the filter icon (Show search options).
2. Define conditions, for example:
   - **From**: `billing@vendor.com`
   - **Subject contains**: `invoice`
   - **Has the words**: `contract OR payment`
3. Click **Create filter**.
4. Check **Forward it to** and select your verified AI assistant mailbox.
5. Optional: also check
   - **Apply the label** (e.g., `Forwarded-to-AI`)
   - **Never send it to Spam**
6. Click **Create filter**.

### 4) Recommended filter patterns
- Finance emails: from specific vendor domains + invoice keywords.
- Support escalation emails: subject contains `urgent`, `escalation`, `SLA`.
- Executive summary flows: only emails with label `AI-Process`.

### 5) Safety recommendations
- Do not forward highly sensitive credentials, private keys, or OTP codes.
- Start with narrow filters first, then gradually expand.
- Use a dedicated label to audit what was forwarded.

### 6) Troubleshooting
- **No forwarding option available**: ensure forwarding address verification is complete.
- **Email not forwarded**: check filter criteria and Spam/Promotions behavior.
- **Forwarding stopped**: verify Gmail account security alerts and forwarding settings.
