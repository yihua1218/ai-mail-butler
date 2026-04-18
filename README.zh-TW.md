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

## 授權條款

本軟體使用 The Unlicense 或 CC0 1.0 Universal 發布，詳細內容請參閱 `LICENSE` 檔案。
