# 待辦事項 (TODO)

## 高優先級
- [x] 在 `src/mail/mod.rs` 中實作輕量級 Rust 內建 SMTP 伺服器（使用 `samotop`）。
- [x] 在 `src/db/mod.rs` 中設定資料庫連線與 `sqlx` 遷移 (Migrations)。
- [x] 在 `src/web/mod.rs` 中建置 Axum Web 伺服器與 API 路由。
- [x] 初始化 React 前端專案，並導入 Ant Design（Apple 風格美學）。

## 中優先級
- [x] 實作 AI 對話功能 (WebSocket 或 REST)。
- [x] 實作郵件解析器，提取轉寄內容與原始寄件者身分。
- [x] 開發 AI 整合模組，用於郵件翻譯與自動回覆。
- [x] 建立支援 HTML/純文字雙格式與雙語的 Email Magic-link 登入流程。
- [x] 加入自動化啟動診斷與資料庫架構自我修復功能。
- [x] 實作使用者可自訂的郵件格式設定 (HTML/純文字/雙格式)。
- [x] 實作訪客模式設定（匿名暱稱儲存於瀏覽器）。
- [ ] 新增單元測試與整合測試。
- [x] 優化 Docker 建置流程，將前端靜態資源打包納入。
