# Google Workspace Admin: Forwarding Allowlist and Policy Controls

This advanced guide helps Google Workspace administrators avoid forwarding failures caused by organizational policies when users forward emails to AI Mail Butler.

## 1) Scope and Goal
Use this guide when:
- Users are in a Google Workspace domain (not personal Gmail only).
- Forwarding to `assistant@mail.example.com` is blocked or silently dropped.

Goal:
- Allow policy-compliant forwarding to AI assistant mailbox.
- Reduce false blocks while keeping security controls in place.

## 2) Pre-check List
Before changing admin policies, confirm:
1. AI assistant mailbox domain has valid MX records and can receive email.
2. The forwarding target address is verified by users in Gmail settings.
3. SPF/DKIM/DMARC for your sending domain are correctly configured.

## 3) Recommended Admin Console Controls
Path references may differ by Google Workspace edition/UI updates.

### A. Add forwarding destination to allowlist / approved routing scope
1. Open **Google Admin console** -> **Apps** -> **Google Workspace** -> **Gmail**.
2. Locate routing/compliance settings related to forwarding and external recipients.
3. Add AI assistant mailbox/domain to approved list (for example):
   - `assistant@mail.example.com`
   - `mail.example.com`
4. Apply to target organizational units (OU) first, then expand.

### B. Configure restricted routing for controlled forwarding
1. Use **Routing** or **Default routing** rule.
2. Create a controlled route for messages matching forwarding use case.
3. Restrict destination to approved assistant domain/mailbox.
4. Keep anti-spam and malware checks enabled.

### C. Tune outbound gateway / external recipient restrictions
If your organization restricts external recipients:
1. Add exception for AI assistant mailbox/domain.
2. Ensure outbound gateway policy does not rewrite/drop forwarded messages unexpectedly.

## 4) Security Baseline (Recommended)
1. Enable DLP checks for highly sensitive categories before forwarding.
2. Block forwarding of credentials, OTP, secrets, and private keys.
3. Keep an audit label in Gmail filters (example: `Forwarded-to-AI`).
4. Enable admin audit logs for routing/policy changes.

## 5) Operational Rollout Strategy
1. Pilot with one OU/team.
2. Use narrow Gmail filters first (specific senders/subjects/labels).
3. Monitor delivery success and false positives for 3-7 days.
4. Expand gradually and document approved forwarding patterns.

## 6) Troubleshooting Matrix
- Symptom: User cannot add forwarding address
  - Check: Domain policy blocks external forwarding verification emails.
- Symptom: Filter exists but no forwarding happens
  - Check: Routing rule precedence and restricted recipient policy.
- Symptom: Some forwarded emails missing attachments
  - Check: Content compliance, DLP, and attachment-type restrictions.
- Symptom: Intermittent drops
  - Check: DMARC/SPF failures, quarantine rules, or gateway timeouts.

## 7) Change Control Checklist
1. Record policy owner and approval ticket ID.
2. Record OU scope and effective time.
3. Record rollback steps.
4. Notify affected users with filter best-practice examples.
