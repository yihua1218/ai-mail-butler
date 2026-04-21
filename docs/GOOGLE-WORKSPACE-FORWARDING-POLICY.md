# Google Workspace Admin: Forwarding Allowlist and Policy Controls / Google Workspace 管理員：轉寄白名單與政策限制

This advanced guide helps Google Workspace administrators avoid forwarding failures caused by organizational policies when users forward emails to AI Mail Butler.

本進階指南協助 Google Workspace 管理員避免因組織政策限制，導致使用者無法將信件轉寄至 AI Mail Butler。

---

## English Instructions

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

---

## 繁體中文說明

## 1) 適用情境與目標
以下情境建議使用本指南：
- 使用者屬於 Google Workspace 企業網域（非單純個人 Gmail）。
- 轉寄到 `assistant@mail.example.com` 被政策擋下或默默失敗。

目標：
- 在符合公司政策前提下允許轉寄到 AI 助理信箱。
- 降低誤擋，同時維持既有安全控制。

## 2) 變更前檢查
調整管理政策前，請先確認：
1. AI 助理信箱網域 MX 設定正確且可收信。
2. 使用者已在 Gmail 設定完成轉寄地址驗證。
3. 寄件網域 SPF/DKIM/DMARC 設定正常。

## 3) 建議在 Admin Console 的控制項
不同版本/介面名稱可能稍有差異。

### A. 將轉寄目標加入白名單 / 核准路由範圍
1. 進入 **Google Admin console** -> **Apps** -> **Google Workspace** -> **Gmail**。
2. 找到與外部轉寄/外部收件者限制相關的路由與合規設定。
3. 將 AI 助理信箱或網域加入核准清單，例如：
   - `assistant@mail.example.com`
   - `mail.example.com`
4. 建議先對特定 OU 套用，確認後再擴大。

### B. 以受控路由方式放行轉寄
1. 使用 **Routing** 或 **Default routing** 規則。
2. 建立符合轉寄情境的受控路由。
3. 目的地限制為核准的 AI 助理網域/信箱。
4. 垃圾信與惡意軟體檢查請維持啟用。

### C. 檢查外部收件者限制 / Outbound gateway
若組織有外寄限制：
1. 對 AI 助理信箱/網域加入例外。
2. 確認 outbound gateway 政策不會改寫或丟棄轉寄信。

## 4) 安全基線（建議）
1. 先套 DLP 規則再放行高敏感類別轉寄。
2. 禁止轉寄憑證、OTP、密鑰、私鑰等內容。
3. 在 Gmail 篩選器加上稽核標籤（例如：`Forwarded-to-AI`）。
4. 啟用管理員稽核日誌，追蹤路由與政策變更。

## 5) 上線策略
1. 先在單一 OU/團隊試行。
2. 先用嚴格篩選條件（寄件者/主旨/標籤）再逐步放寬。
3. 觀察 3-7 天送達率與誤擋率。
4. 逐步擴大並文件化核准轉寄樣式。

## 6) 常見問題排查
- 現象：使用者無法新增轉寄地址
  - 檢查：網域政策是否擋下外部轉寄驗證信。
- 現象：篩選器存在但未轉寄
  - 檢查：路由規則優先序與外部收件者限制。
- 現象：部分信件附件不見
  - 檢查：內容合規、DLP、附件類型限制。
- 現象：偶發性轉寄失敗
  - 檢查：DMARC/SPF 驗證、隔離規則、gateway timeout。

## 7) 變更管理清單
1. 記錄政策負責人與核准單號。
2. 記錄 OU 套用範圍與生效時間。
3. 記錄回滾步驟。
4. 對使用者發布篩選器最佳實務通知。
