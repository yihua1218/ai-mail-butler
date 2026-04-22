# AI Mail Butler (AI 郵件助理)

AI Mail Butler 是一個自託管的 AI 郵件處理助理平台。它由專屬的郵件伺服器 (Mail Server) 與網頁介面 (Web GUI) 組成。使用者只需從自己的郵件伺服器將選定的郵件轉寄到助理的專屬信箱，助理就能自動識別發件人、透過電子郵件進行新用戶引導 (Onboarding)，並執行 AI 驅動的操作（如：翻譯、自動回覆）。此外，還提供了一個透明的 Web 儀表板，讓使用者隨時監控所有被處理的郵件和執行動作。

## 系統架構

- **後端**：Rust (`tokio`, `axum`)
- **資料庫**：SQLite (透過 `sqlx`)
- **AI 整合**：透過 HTTP 調用自定義 LLM 端點 (`reqwest`)
- **郵件處理**：SMTP 接收 (Samotop) 與發送 (mail-send)，支援 MIME 與 HTML 格式
- **穩定性**：具備自動化啟動診斷與資料庫自我修復機制

## 功能特點

- **郵件轉寄偵測與解析**：自動識別轉寄郵件的原始使用者身份。
- **AI 處理模組**：支援英翻繁中、基於規則的自動回覆。
- **AI 聊天介面**：透過 Web UI 與您的郵件助理直接對話。匿名訪客亦可與 AI 交流以了解系統。提供詳細的對話統計（Token 數量與生成速度）。
- **文件知識回答**：AI 助理可根據提問內容，從專案文件中檢索相關資訊後再回覆，降低憑空回答風險。
- **文件快取索引**：文件檢索使用記憶體索引並定期刷新，在維持內容新鮮度的同時減少重複磁碟掃描。
- **對話建立規則**：使用者可直接在聊天中提出需求，助理會抽取可執行規則並自動建立（含重複規則防護）。
- **長期記憶機制**：AI 助理具備長期記憶，能根據過往對話脈絡提供更精確且個性化的服務。
- **互動式 Onboarding**：新用戶登入後，AI 助理會主動引導一系列問答設定，幫助用戶快速上手。
- **自訂 AI 助理身份**：使用者可自訂助理的中英文名稱與回覆語氣（如：專業、親切等）。
- **數據化優化**：系統自動統計使用者行為與重複提問次數，作為助理服務優化的依據。
- **使用者配置與註冊**：透過郵件完成自動化註冊流程。
- **角色權限控制 (RBAC)**：支援管理員 (Admin)、一般用戶 (User) 與匿名角色，具備嚴格的資料隔離。
- **無密碼身分驗證**：透過 Email 寄送 Magic link 登入（支援雙語純文字與 HTML 富文本）。
- **使用者設定**：可切換「自動回覆」與「試運行 (Dry Run)」模式（AI 回覆會先寄給自己審核）。
- **Web 儀表板**：查看郵件處理歷史與採取的行動，採用基於 Ant Design 的 Apple 風格美學設計。
- **內建 SMTP**：輕量級 Rust 原生 SMTP 伺服器，部署簡單。
- **系統診斷**：啟動時自動驗證並修復資料庫結構 (Self-healing)。
- **唯讀疊加模式（Readonly Overlay Mode）**：對現有資料快照執行系統，不對其做任何修改。所有寫入會被導向獨立的 overlay 目錄；讀取時若 overlay 無對應檔案，自動 fallback 到 base 快照。適用於 Demo、Staging 或唯讀鏡像部署。

## 開發與測試

### 系統需求
- Rust (cargo)

### 編譯
```bash
cargo build
```

### 執行
```bash
cargo run
```

### CLI 除錯模式（不啟動後端伺服器）
可使用 CLI 模式直接處理本地 `spool` 目錄中的 `.eml` 檔案，不啟動 SMTP/Web server。

單次處理（預設）：
```bash
cargo run -- --mode cli
```

指定 spool 目錄並輸出 JSON 報表：
```bash
cargo run -- --mode cli --spool-dir data/mail_spool --report-json data/mail_spool/cli-report.json
```

保留原始 `.eml` 檔案（方便重複除錯）：
```bash
cargo run -- --mode cli --keep-files
```

持續監看模式：
```bash
cargo run -- --mode cli --watch
```

互動式 REPL 模式：
```bash
cargo run -- --mode cli --repl
```
REPL 指令：`list`、`show <index|path>`、`process <index|path>`、`retry-unknown`、`report`、`exit`。

處理單一指定 `.eml` 檔案：
```bash
cargo run -- --mode cli --eml-file /absolute/path/to/mail_123.eml --keep-files
```

啟用 AI Agent 模擬（規則 + 記憶）並顯示 step-by-step：
```bash
cargo run -- --mode cli \
    --eml-file /absolute/path/to/mail_123.eml \
    --simulate-agent --simulate-rules --simulate-memory \
    --as-user user@example.com \
    --step --keep-files
```

參數說明：
- `--simulate-agent`：開啟模擬流程。
- `--simulate-rules`：套用啟用中的 `email_rules`，並產生模擬自動回覆預覽。
- `--simulate-memory`：讀取 `user_memories`，產生含記憶脈絡的模擬回覆預覽。
- `--as-user`：當寄件者無法映射使用者時，強制指定使用者情境以便除錯。
- `--step`：在 CLI 直接顯示每個處理步驟與模擬進度。

可選文件檢索控制：
- `DOCS_WHITELIST`：以逗號分隔可被 AI 引用的文件檔名或關鍵字。例如：`DOCS_WHITELIST=GMAIL-SMTP-SETUP.md,zh-TW`
- 語言偏好效果：登入使用者若設定 `preferred_language=zh-TW`，系統會優先命中 `*.zh-TW.md` 文件內容。

### 測試
```bash
cargo test
```

### Docker 部署
使用 Docker Compose 進行建置與背景執行：
```bash
docker-compose up --build -d
```

詳細的部署指南（含 AWS EC2 實作）：
- [Docker 部署指南 (繁體中文)](DOCKER_AWS_GUIDE.zh-TW.md)
- [Docker Deployment Guide (English)](DOCKER_AWS_GUIDE.md)

## Cloudflare DNS 設定 (MX 紀錄)

為了讓您的 AI Mail Butler 能夠順利接收自訂網域的電子郵件，您需要設定 DNS 紀錄。以下是以在 Cloudflare 設定 `mail.example.com` 為例的教學：

1. 登入 Cloudflare 儀表板，並選擇您的網域（例如：`example.com`）。
2. 進入 **DNS** -> **Records (紀錄)** 頁面。
3. 首先，請確保您有一個 `A` 紀錄指向您伺服器的 IP 位址：
   - **類型 (Type)**：`A`
   - **名稱 (Name)**：`mail` (或是您自訂的子網域)
   - **IPv4 位址 (IPv4 address)**：`您的伺服器_IP`
   - **Proxy 狀態 (Proxy status)**：僅限 DNS / DNS only（**請務必關閉橘色雲朵**，因為 Cloudflare 的 Proxy 只支援 HTTP/HTTPS，不支援 SMTP 協定）。
4. 新增 `MX` 紀錄，將郵件導向您的伺服器：
   - **類型 (Type)**：`MX`
   - **名稱 (Name)**：`mail` (這代表您將接收寄給 `*@mail.example.com` 的信件)
   - **郵件伺服器 (Mail server)**：`mail.example.com`
   - **優先權 (Priority)**：`10`

等候 DNS 生效後，任何寄到 `anything@mail.example.com` 的信件都會被成功轉發到您的 AI Mail Butler 伺服器了。

## SMTP 寄信設定 (Gmail / M365)

為了發送登入連結 (Magic Link) 與 AI 自動回覆，您需要設定 SMTP 轉發伺服器。

### Gmail
- [Gmail SMTP 設定指南 (繁中/EN)](docs/GMAIL-SMTP-SETUP.md)
- [Gmail 篩選器與轉寄設定指南 (繁中)](docs/GMAIL-FILTER-FORWARDING.zh-TW.md)
- [Google Workspace 管理員轉寄白名單與政策指南 (繁中)](docs/GOOGLE-WORKSPACE-FORWARDING-POLICY.zh-TW.md)

### Microsoft 365 (M365)
如果您計畫使用 Microsoft 365 (M365) 帳戶來寄信，您有兩種選擇：

1.  **應用程式密碼 (SMTP AUTH)**：適合個人或小規模使用，設定最快。
    - [M365 SMTP 設定指南 (繁中/EN)](docs/SMTP-SETUP.md)
2.  **OAuth 2.0 (Microsoft Graph)**：生產環境推薦，若組織禁用了 SMTP AUTH 則必須使用此方式。
    - [繁體中文 OAuth 設定指南](docs/M365_OAUTH_SETUP.zh-TW.md)
    - [English OAuth Setup Guide](docs/M365_OAUTH_SETUP.md)

## 唯讀疊加模式（Readonly Overlay Mode）

對現有資料快照執行 AI Mail Butler，而不對其進行任何修改。所有寫入操作會被重導向至 overlay 目錄；Spool 與檔案讀取在 overlay 無對應檔案時，會自動透明地 fallback 至 base 快照。API 層同時被鎖定為唯讀操作。

- [唯讀疊加模式說明 (繁體中文)](docs/READONLY-OVERLAY-MODE.zh-TW.md)
- [Readonly Overlay Mode Guide (English)](docs/READONLY-OVERLAY-MODE.md)

## 角色存取控制 (RBAC)

系統支援嚴格的角色權限與資料隔離。關於如何設定管理員，以及每種角色所能看見的資料範圍，請詳閱 [角色存取控制 (RBAC) 說明文件](docs/RBAC.zh-TW.md)。

## 授權條款 (License)

本軟體使用 The Unlicense 或 CC0 1.0 Universal 發布，詳細內容請參閱 `LICENSE` 檔案。
