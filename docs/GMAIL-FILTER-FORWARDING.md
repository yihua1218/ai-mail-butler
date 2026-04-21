# Gmail Filter and Forwarding Setup for AI Mail Butler / Gmail 篩選器與轉寄到 AI Mail Butler 設定指南

This guide explains how to:
1. Verify your AI assistant mailbox as a forwarding target in Gmail.
2. Create Gmail filters to auto-forward only the emails you want AI Mail Butler to process.

本指南說明如何：
1. 在 Gmail 中先驗證 AI 助理信箱作為可轉寄目標。
2. 建立 Gmail 篩選器，僅自動轉寄你要交給 AI Mail Butler 處理的信件。

---

## English Instructions

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

---

## 繁體中文說明

### 1) 前置條件
- 你的 AI Mail Butler 助理信箱已可用（例如：`assistant@mail.example.com`）。
- 該信箱可正常收信。

### 2) 在 Gmail 新增並驗證轉寄目標
1. 打開 Gmail，點右上角齒輪 -> **查看所有設定**。
2. 進入 **轉寄和 POP/IMAP**。
3. 點選 **新增轉寄地址**。
4. 輸入 AI 助理信箱，例如 `assistant@mail.example.com`。
5. Gmail 會寄一封驗證信到該信箱。
6. 打開驗證信並點擊確認連結（或把驗證碼填回 Gmail）。

注意：轉寄地址未驗證完成前，Gmail 不會執行自動轉寄。

### 3) 建立篩選器做精準轉寄
1. 在 Gmail 搜尋列點擊篩選器圖示（顯示搜尋選項）。
2. 設定條件，例如：
   - **寄件者**：`billing@vendor.com`
   - **主旨包含**：`invoice`
   - **包含字詞**：`contract OR payment`
3. 點 **建立篩選器**。
4. 勾選 **將其轉寄到**，並選擇已驗證的 AI 助理信箱。
5. 可選建議：
   - 同時勾選 **套用標籤**（例如 `Forwarded-to-AI`）
   - 勾選 **不要將郵件傳送至垃圾郵件**
6. 點 **建立篩選器** 完成。

### 4) 建議篩選策略
- 財務信件：限定特定供應商網域 + 發票關鍵字。
- 客訴/升級案件：主旨含 `urgent`、`escalation`、`SLA`。
- 高層摘要流程：只轉寄有 `AI-Process` 標籤的信件。

### 5) 安全建議
- 請勿轉寄高敏感憑證、私鑰、一次性驗證碼（OTP）。
- 先從嚴格篩選條件開始，再逐步放寬。
- 建議套標籤，方便追蹤哪些信件已轉給 AI。

### 6) 常見問題排查
- **看不到轉寄選項**：先確認轉寄地址驗證已完成。
- **信件沒有被轉寄**：檢查篩選條件與垃圾郵件/促銷分頁行為。
- **轉寄突然失效**：檢查 Gmail 帳號安全通知與轉寄設定狀態。
