# 待辦事項 (TODO)

## 高優先級
- [ ] 在 `src/mail/mod.rs` 中實作輕量級 Rust 內建 SMTP 伺服器（使用 `samotop`）。
- [ ] 在 `src/db/mod.rs` 中設定資料庫連線與 `sqlx` 遷移 (Migrations)。
- [ ] 在 `src/web/mod.rs` 中建置 Axum Web 伺服器與 API 路由。
- [ ] 初始化 React 前端專案，並導入 Ant Design（Apple 風格美學）。

## 中優先級
- [ ] 實作 AI 對話功能 (WebSocket 或 REST)。
- [ ] 實作郵件解析器，提取轉寄內容與原始寄件者身分。
- [ ] 開發 AI 整合模組，用於郵件翻譯與自動回覆。
- [ ] 建立 Email Magic-link 無密碼登入驗證流程。

## 低優先級
- [ ] 新增單元測試與整合測試。
- [ ] 優化 Docker 建置流程，將前端靜態資源打包納入。
