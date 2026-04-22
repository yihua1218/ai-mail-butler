# AI Mail Butler 運作說明與功能介紹

本文件介紹 AI Mail Butler 的核心工作流程、目前支援的信件處理功能，以及「**功能許願牆**」——讓已註冊的使用者提出建議並投票決定下一步要開發的能力。

---

## 概觀

AI Mail Butler 作為智慧型信件處理代理人。它不直接連接到您現有的信箱，而是讓您選擇性地將特定信件**轉寄**至 AI 助理的專屬信箱，助理再依您定義的規則進行處理。

```
您的收件匣
    │
    │  (郵件轉寄篩選器 — 只需設定一次)
    ▼
AI 助理信箱   ←── 接收轉寄進來的信件
    │
    ▼
AI 處理引擎
    ├──▶  自動回信         (依規則草擬或直接寄出回覆)
    ├──▶  帳務整理         (萃取並彙整財務資訊)
    └──▶  更多功能…        (在下方許願牆投票敲碗)
```

---

## 三步驟快速上手

### 第一步 — 取得 AI 助理信箱地址

登入後，前往**關於**頁面（`/about`），複製您的專屬 AI 助理電子郵件地址。

範例：`assistant@mail.your-domain.com`

### 第二步 — 在您的郵件服務設定轉寄規則

在 Gmail、Outlook 或任何支援轉寄篩選的郵件服務中：

1. 將 AI 助理信箱新增為已驗證的轉寄地址。
2. 建立篩選條件（例如：寄件者網域、主旨關鍵字、標籤），將符合條件的信件自動轉寄至 AI 信箱。

詳細設定指南：
- [Gmail 篩選器與轉寄設定指南 (繁中)](GMAIL-FILTER-FORWARDING.zh-TW.md)
- [Gmail Filter & Forwarding Setup (英文)](GMAIL-FILTER-FORWARDING.md)
- [Google Workspace 管理員轉寄政策指南 (繁中)](GOOGLE-WORKSPACE-FORWARDING-POLICY.zh-TW.md)

### 第三步 — 定義信件處理規則

在**規則**頁面（`/rules`）或直接透過 **AI 聊天**（`/chat`），告訴助理如何處理各類信件。範例：

- 「如果是 vendor.com 寄來的帳單通知，請歸檔到財務報表。」
- 「如果主旨包含『合約續約』，請草擬一封請對方確認的回覆。」

規則可手動新增，也可直接在聊天中自然口語描述——助理會自動從對話中抽取可執行規則並建立。

---

## 目前支援的功能

### 自動回信

AI 助理會分析每封進來的信件內容，並依您的規則自動產生回覆草稿。您可選擇：
- **試運行 (Dry Run) 模式**：AI 回覆先寄給自己審核，確認無誤後再手動發送。
- **全自動模式**：AI 回覆直接寄給原始寄件者。

**可自訂項目：**
- 助理的中文 / 英文顯示名稱
- 回覆語氣（專業、親切、簡潔等）
- 自動偵測回覆語言或固定語言

### 帳務整理

助理會自動從電子郵件中萃取財務相關資訊，包含發票、銀行對帳單、信用卡帳單、繳費通知等，並彙整成月度財務報表，可在**財務**頁面（`/finance`）查看。

**可萃取的資訊欄位：**
- 交易金額與幣別
- 繳費截止日
- 帳單金額
- 發卡銀行與卡號末四碼
- 財務類別（收入 / 支出 / 帳單）
- 月份鍵（用於月度彙整）

---

## 功能許願牆

**功能介紹**頁面（`/how-it-works`）包含社群**許願牆**，已登入使用者可以：

1. **瀏覽**現有功能建議（官方路線圖項目 + 社群提案）。
2. **投票**支持他們最想要的功能——每人每個功能限投一票，可隨時取消。
3. **提交**自己的功能建議，填寫名稱與選填說明即可。

票數越高的功能，優先納入開發排程。

### API 端點

| 方法   | 路徑                        | 說明                                                                                    |
| ------ | --------------------------- | --------------------------------------------------------------------------------------- |
| `GET`  | `/api/wishes?email=<email>` | 取得所有許願，附票數。若提供 `email` 且使用者已登入，每筆會回傳 `user_has_voted` 欄位。 |
| `POST` | `/api/wishes`               | 提交新許願。請求體：`{ email, title, description? }`                                    |
| `POST` | `/api/wishes/:id/vote`      | 切換投票。請求體：`{ email }` — 尚未投票則新增，已投票則刪除（切換式）。                |

### 預植官方許願項目

系統啟動時會自動植入以下項目（冪等操作，重啟不會重複新增）：

| ID                          | 標題                                      | 狀態     |
| --------------------------- | ----------------------------------------- | -------- |
| `official-auto-reply`       | 自動回信 / Auto Reply                     | ✅ 已上線 |
| `official-bill-accounting`  | 帳務整理 / Bill & Finance Accounting      | ✅ 已上線 |
| `wish-smart-labels`         | 智慧分類標籤 / Smart Label Classification | 🗳️ 投票中 |
| `wish-meeting-summary`      | 會議邀請摘要 / Meeting Invitation Summary | 🗳️ 投票中 |
| `wish-subscription-tracker` | 訂閱追蹤 / Subscription Tracker           | 🗳️ 投票中 |

---

## 資料庫結構

新增兩張 SQLite 資料表：

```sql
-- 功能許願清單
CREATE TABLE feature_wishes (
    id          TEXT PRIMARY KEY NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    created_by  TEXT,              -- 官方/預植項目為 NULL
    is_official BOOLEAN NOT NULL DEFAULT 0,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 每位使用者對每個許願只能有一票（UNIQUE 約束強制執行）
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

兩張表均以 `CREATE TABLE IF NOT EXISTS` 冪等建立，可安全地對現有資料庫執行。

---

## 安全說明

- **寫入操作需要身份驗證**：提交許願和投票都需要提供已在資料庫中存在的有效 `email`，未註冊的 email 會被回應 `401 Unauthorized`。
- **輸入驗證**：許願標題最長 200 字；空白標題在前端（表單驗證）與後端（API 守衛）兩層均會被拒絕。
- **切換投票具冪等性**：同一使用者對同一許願重複提交投票，會刪除而非重複新增，防止票數膨脹。
- **目前許願/投票端點未設置速率限制**。若對外公開部署時有濫用疑慮，建議另行新增頻率限制。

---

## 相關文件

- [Gmail 篩選器與轉寄設定指南](GMAIL-FILTER-FORWARDING.zh-TW.md)
- [Google Workspace 管理員轉寄政策指南](GOOGLE-WORKSPACE-FORWARDING-POLICY.zh-TW.md)
- [M365 OAuth 設定指南](M365_OAUTH_SETUP.zh-TW.md)
- [角色存取控制 (RBAC) 說明](RBAC.zh-TW.md)
