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
- **AI 聊天介面**：透過 Web UI 與您的郵件助理直接對話。匿名訪客亦可與 AI 交流以了解系統。
- **使用者配置與註冊**：透過郵件完成自動化註冊流程。
- **角色權限控制 (RBAC)**：支援管理員 (Admin)、一般用戶 (User) 與匿名角色，具備嚴格的資料隔離。
- **無密碼身分驗證**：透過 Email 寄送 Magic link 登入（支援雙語純文字與 HTML 富文本）。
- **使用者設定**：可切換「自動回覆」與「試運行 (Dry Run)」模式（AI 回覆會先寄給自己審核）。
- **Web 儀表板**：查看郵件處理歷史與採取的行動，採用基於 Ant Design 的 Apple 風格美學設計。
- **內建 SMTP**：輕量級 Rust 原生 SMTP 伺服器，部署簡單。
- **系統診斷**：啟動時自動驗證並修復資料庫結構 (Self-healing)。回 console log 模式，不影響正常啟動。

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

### 測試
```bash
cargo test
```

### Docker 部署
使用 Docker Compose 進行建置與背景執行：
```bash
docker-compose up --build -d
```

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

### Microsoft 365 (M365)
如果您計畫使用 Microsoft 365 (M365) 帳戶來寄信，您有兩種選擇：

1.  **應用程式密碼 (SMTP AUTH)**：適合個人或小規模使用，設定最快。
    - [M365 SMTP 設定指南 (繁中/EN)](docs/SMTP-SETUP.md)
2.  **OAuth 2.0 (Microsoft Graph)**：生產環境推薦，若組織禁用了 SMTP AUTH 則必須使用此方式。
    - [繁體中文 OAuth 設定指南](docs/M365_OAUTH_SETUP.zh-TW.md)
    - [English OAuth Setup Guide](docs/M365_OAUTH_SETUP.md)

## 角色存取控制 (RBAC)

系統支援嚴格的角色權限與資料隔離。關於如何設定管理員，以及每種角色所能看見的資料範圍，請詳閱 [角色存取控制 (RBAC) 說明文件](docs/RBAC.zh-TW.md)。

## 授權條款 (License)

本軟體使用 The Unlicense 或 CC0 1.0 Universal 發布，詳細內容請參閱 `LICENSE` 檔案。
