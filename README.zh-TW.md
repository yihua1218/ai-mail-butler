# AI Mail Butler (AI 郵件助理)

AI Mail Butler 是一個自託管的 AI 郵件處理助理平台。它由專屬的郵件伺服器 (Mail Server) 與網頁介面 (Web GUI) 組成。使用者只需從自己的郵件伺服器將選定的郵件轉寄到助理的專屬信箱，助理就能自動識別發件人、透過電子郵件進行新用戶引導 (Onboarding)，並執行 AI 驅動的操作（如：翻譯、自動回覆）。此外，還提供了一個透明的 Web 儀表板，讓使用者隨時監控所有被處理的郵件和執行動作。

## 系統架構

- **後端 (Backend)**: Rust (`tokio`, `axum`)
- **資料庫 (Database)**: SQLite (透過 `sqlx`)
- **AI 整合 (AI Integration)**: 透過 HTTP 串接自訂的 LLM API (`reqwest`)
- **郵件處理 (Mail Handling)**: 郵件接收與發送 (`lettre`, `mailparse` 等)

## 核心功能

- **轉寄偵測與解析**：準確識別轉寄該郵件的原始使用者。
- **AI 處理模組**：支援英文自動翻譯為繁體中文，以及基於規則的自動回覆功能。
- **AI 對話介面**：透過 Web UI 上的互動式介面直接與您的電子郵件助理對話。
- **用戶設定與引導 (Onboarding)**：透過電子郵件自動完成新使用者的引導流程。
- **無密碼登入 (Passwordless)**：採用 Magic Link 的登入機制，安全且便捷。
- **Web 儀表板**：檢視接收郵件歷史與處理紀錄，UI 採用啟發自 Ant Design 的 Apple 風格美學。
- **內建 SMTP 伺服器**：輕量級且原生的 Rust SMTP 伺服器，部署更為簡便。

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

為了讓您的 AI Mail Butler 能夠順利接收自訂網域的電子郵件，您需要設定 DNS 紀錄。以下是以在 Cloudflare 設定 `mail.yihua.app` 為例的教學：

1. 登入 Cloudflare 儀表板，並選擇您的網域（例如：`yihua.app`）。
2. 進入 **DNS** -> **Records (紀錄)** 頁面。
3. 首先，請確保您有一個 `A` 紀錄指向您伺服器的 IP 位址：
   - **類型 (Type)**：`A`
   - **名稱 (Name)**：`mail` (或是您自訂的子網域)
   - **IPv4 位址 (IPv4 address)**：`您的伺服器_IP`
   - **Proxy 狀態 (Proxy status)**：僅限 DNS / DNS only（**請務必關閉橘色雲朵**，因為 Cloudflare 的 Proxy 只支援 HTTP/HTTPS，不支援 SMTP 協定）。
4. 新增 `MX` 紀錄，將郵件導向您的伺服器：
   - **類型 (Type)**：`MX`
   - **名稱 (Name)**：`mail` (這代表您將接收寄給 `*@mail.yihua.app` 的信件)
   - **郵件伺服器 (Mail server)**：`mail.yihua.app`
   - **優先權 (Priority)**：`10`

等待 DNS 生效後，任何寄到 `anything@mail.yihua.app` 的信件都會被成功轉發到您的 AI Mail Butler 伺服器了。

## 授權條款

本軟體使用 The Unlicense 或 CC0 1.0 Universal 發布，詳細內容請參閱 `LICENSE` 檔案。
