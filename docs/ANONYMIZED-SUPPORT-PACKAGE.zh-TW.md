# 去識別化支援與測試資料包規劃

## 目標
提供一個由使用者主動發起、送出前可 review 的資料包流程，讓使用者可以從 Dashboard 或其他能看到信件與 log 的畫面，選擇「可成功處理的信件」、「未成功處理的信件」或「error log」，產生一份已將個人敏感資料替換成虛擬身份的測試驗證資料。

這份資料包可用於：

- 開發人員重現使用者回報的處理問題。
- 驗證修復是否真的解決特定信件或錯誤 log。
- 日後沉澱成自動化測試 fixtures，降低同類回歸風險。

## 核心原則

1. **使用者主動選取**  
   系統不自動收集完整 mailbox。使用者必須在 Dashboard 選定要納入的信件或 log。

2. **先去識別化，再 review**  
   後端產生資料包時先做去識別化，前端只顯示去識別化後的內容。使用者 review 後才決定是否下載或交給開發人員。

3. **虛擬身份需一致**  
   同一份資料包內，相同 email、電話、token 等敏感值會被替換成一致的虛擬值，例如 `person1@example.test`、`+1-555-0101`。這保留測試所需的關聯性，但移除真實身份。

4. **不長期保存資料包**  
   第一階段資料包即時產生並回傳給瀏覽器，不寫入資料庫。若未來需要「送給開發人員」的伺服器端收件箱，應另加保留期限、存取審計與刪除機制。

5. **可作為測試 fixture**  
   輸出格式應穩定、可 diff、可被自動化測試讀取。第一階段使用 JSON bundle，後續可包成 zip 並加入原始 MIME 檔、快照 metadata 或測試 runner manifest。

## 使用流程

### Dashboard

1. 使用者在「你的信件」表格選取一筆或多筆信件。
2. 使用者在「Mail Server Logs」表格選取一筆或多筆 log。
3. 點選「建立去識別化測試資料包」。
4. 系統呼叫後端產生去識別化 bundle。
5. 前端顯示預覽 modal：
   - 資料包摘要：信件數、log 數、processing log 數。
   - 虛擬身份對照摘要：只顯示虛擬值，不顯示原始值。
   - JSON 預覽：完整去識別化資料。
6. 使用者確認內容後，下載 JSON 檔並自行提供給開發人員。

### 成功與失敗信件

- 成功處理信件可由 `emails.status` 判斷，例如 `processed`、`drafted`、`replied`。
- 未成功處理信件可透過 `pending` 狀態、手動選取的 error log，或 error log 的 `context` 對應到 spool `.eml`。
- 第一階段不強制自動判斷成功/失敗；以使用者在 Dashboard 選取的列為準，bundle 中保留 `status`、`error_type`、`result` 等欄位供測試判斷。

## 去識別化規則

第一階段使用保守的 deterministic pseudonymization：

| 類型             | 替換範例                | 備註                                              |
|------------------|-------------------------|---------------------------------------------------|
| Email            | `person1@example.test`  | 同一原始 email 在同一 bundle 中保持同一虛擬值     |
| 電話             | `+1-555-0101`           | 適用常見 US/TW 電話格式                           |
| 長 token/API key | `token_1_redacted`      | 避免測試包洩漏憑證                                |
| 信用卡或長數字   | `4111111111110001`      | 保留測試格式，但不可付款                           |
| URL host         | `service1.example.test` | 後續階段補強；第一階段先處理文字中的 email、電話、token、長數字 |

仍需讓使用者 review，因為自由文字中可能包含系統未知的姓名、地址、公司內部代號或截圖 OCR 文字。Preview modal 必須清楚提醒使用者「下載前請確認沒有真實個資」。

## Bundle 格式

```json
{
  "schema_version": 1,
  "generated_at": "2026-05-10T12:00:00Z",
  "requester": "person1@example.test",
  "summary": {
    "emails": 2,
    "mail_errors": 1,
    "processing_logs": 3
  },
  "identity_summary": {
    "emails": ["person1@example.test", "person2@example.test"],
    "phones": ["+1-555-0101"],
    "tokens": ["token_1_redacted"]
  },
  "emails": [],
  "mail_errors": [],
  "processing_logs": []
}
```

## API 設計

### `POST /api/support-package/preview`

Request:

```json
{
  "email": "user@example.com",
  "email_ids": ["email-id-1"],
  "error_ids": [123]
}
```

Response:

```json
{
  "status": "success",
  "package": { "...": "去識別化 bundle" }
}
```

授權：

- 一般使用者只能匯出自己的 `emails` 與 `mail_errors`。
- admin/developer 可匯出 Dashboard 可見的 error logs，用於支援與除錯。
- 若同一 request 完全沒有選取任何信件或 log，回傳錯誤。

## 未來擴充

- 加入伺服器端「提交給開發人員」流程，搭配保留期限、audit trail、刪除 API。
- 產生 zip：`manifest.json`、`emails/*.json`、`logs/*.json`、`raw/*.eml.redacted`。
- 將已確認的 bundle 存入 `tests/fixtures/support-packages/`，用於 regression tests。
- 增加更完整的 NER/地址/姓名去識別化，但仍維持使用者 review 為最後防線。
- 支援「只匯出最小重現資料」模式，自動納入相關 processing logs、rules、finance extraction records。

## 第一階段實作切點

1. 後端新增 `/api/support-package/preview`，即時查詢選取資料並回傳去識別化 JSON。
2. Dashboard 信件表格沿用既有 row selection。
3. Mail error 表格新增 row selection。
4. Dashboard 新增建立資料包按鈕與 preview modal。
5. 使用者在 modal review 後可下載 JSON 檔。
