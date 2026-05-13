# Unit Test 覆蓋率計畫

## 目標

提高最容易回歸的核心行為測試覆蓋：郵件解析、隱私控制、remote-debug path resolution、手動重新處理，以及部署/安全設定。

## 目前基線

最後更新：2026-05-13

驗證指令：

```bash
cargo test
cargo llvm-cov --summary-only
```

目前結果：

```text
97 passed; 0 failed
```

Coverage 快照：

| 指標                      |   目前數值 |
|---------------------------|-----------:|
| 日期                      | 2026-05-13 |
| Backend line coverage     |     52.72% |
| Backend function coverage |     58.07% |
| Backend region coverage   |     53.06% |
| Frontend coverage         |   尚未量測 |

先前 2026-04-22 的 backend 實測 snapshot：

| 指標                      | 先前數值 |
|---------------------------|---------:|
| Backend line coverage     |   15.41% |
| Backend function coverage |   23.20% |
| Backend region coverage   |   15.80% |

2026-05-13 當天稍早的本地 snapshots：

| Snapshot                                     |   Line | Function | Region |
|----------------------------------------------|-------:|---------:|-------:|
| 補完 config/web helper tests 後              | 29.19% |   29.77% | 27.87% |
| 補完 main/AI parser tests 後                 | 33.00% |   34.43% | 31.86% |
| 補完 mail/web/firewall helper tests 後       | 42.67% |   49.50% | 43.27% |
| 補完 rule chat command tests 後              | 44.65% |   50.33% | 45.57% |
| 補完 dashboard/rules/auto-reply API tests 後 | 48.38% |   53.91% | 48.82% |
| 補完 auth/settings/privacy API tests 後      | 52.72% |   58.07% | 53.06% |

環境備註：

- `cargo llvm-cov --summary-only` 可能需要允許 bind `127.0.0.1`，因為 reprocessing 測試會啟動本機 mock AI HTTP server。

## 測試涵蓋範圍狀態

| 測試範圍                                 | 狀態   | 說明                                                               |
|------------------------------------------|--------|--------------------------------------------------------------------|
| Config 預設值與 env parsing              | 已涵蓋 | 包含 readonly、remote-debug、Cloudflare、SMTP port、whitelist parsing。 |
| `first_email_address` 解析邏輯           | 已涵蓋 | `mail` helper 測試。                                                |
| MIME 附件與 inline 文字分離              | 已涵蓋 | `mail` helper 測試。                                                |
| 財務分類/方向 normalization              | 已涵蓋 | `mail` helper 測試。                                                |
| unmatched rule guidance gating           | 已涵蓋 | `mail` helper 測試。                                                |
| 規則意圖判斷與去重插入                   | 已涵蓋 | `web` 單元測試。                                                    |
| MX 解析 helper 行為                      | 已涵蓋 | `mail` 與 `web` helper 測試。                                       |
| 訓練授權回答解析                         | 已涵蓋 | `web` 單元測試。                                                    |
| 訓練資料脫敏 regex 遮罩                  | 已涵蓋 | `web` 單元測試。                                                    |
| Onboarding 問題遞進                      | 已涵蓋 | `services` 單元測試。                                               |
| Remote-debug SQLite path helpers         | 已涵蓋 | `sqlite_url_to_path`、overlay-relative path contract。               |
| 手動重新處理財務 rollback                | 已涵蓋 | DB 層 web 測試。                                                    |
| 手動重新處理規則命中與草稿重建           | 已涵蓋 | Mock AI + DB 層 web 測試。                                          |
| 手動重新處理 archived source fallback    | 已涵蓋 | 明確 archive path hydration 測試。                                  |
| Processing step API contract             | 已涵蓋 | `key`、`label_key`、`metadata` contract 測試。                        |
| SMTP security whitelist/blocking 行為    | 已涵蓋 | `smtp_security` 測試。                                              |
| Host firewall agent validation           | 已涵蓋 | `firewall_agent` 測試。                                             |
| settings 寫入與 magic-token 驗證         | 已涵蓋 | DB 層 settings 與 magic-token 驗證測試。                            |
| 訓練匯出 API 授權邊界                    | 未涵蓋 | 需補 API 權限測試。                                                 |
| 聊天成功後 transcript 寫入               | 未涵蓋 | 需補 API 流程測試。                                                 |
| GDPR/DSAR 隱私 handler 基本流程          | 已涵蓋 | Consent、DSAR、privacy、age verification、retention API 測試。          |
| GDPR 刪除時清除 `chat_transcripts`       | 未涵蓋 | 需補 transaction/cleanup 測試。                                     |
| Admin runtime API 授權與 payload         | 未涵蓋 | Remote-debug 新增後的重要保護點。                                   |
| Support package pseudonymization helpers | 已涵蓋 | 穩定 pseudonymization 與 identity summary 測試。                    |
| 前端 consent 開關互動與送出              | 未涵蓋 | 需補 Vitest/RTL 測試。                                              |
| Dashboard 多封信重處理 UI state          | 未涵蓋 | 需補各 row 獨立 timeline 的前端測試。                               |

## 本次新增覆蓋

### 2026-05-13

- 新增 `mail` CLI/overlay 測試：
  - sorted `.eml` spool listing。
  - readonly overlay/base file fallback。
  - CLI target resolution by index and name。
  - 從 `mail_errors` requeue unknown-sender。
- 新增更多 `mail` helper 測試：
  - CLI run report counters。
  - runtime path/dir overlay mapping。
  - inline plain/html body decoding。
  - MIME fallback attachment names。
  - login URL generation。
  - fenced and embedded JSON extraction。
- 新增 `web` remote-debug/admin helper 測試：
  - access-mode normalization。
  - runtime mail path remapping。
  - persisted remote-debug access mode。
  - write API enablement posture。
  - remote base DB path fallback。
  - SQLite identifier quoting 與 table column lookup。
- 新增 `web` API/handler 測試：
  - rule chat count/list/edit/disable/delete flows。
  - dashboard anonymous/personal/admin views。
  - rules API create/list/update/toggle/delete。
  - auto-reply draft list/update/delete，並確認 draft body 可讀。
  - magic-token verification 與 settings persistence。
  - consent、DSAR、privacy settings、age verification、retention policy flows。
- 新增 `web` helper 測試：
  - support package pseudonymization。
  - rule label generation 與 AI label sanitization。
  - rule command helper intent parsing。
  - docs query terms 與 best-matching snippets。
  - archived/raw mail rendering helpers。
- 新增 `firewall_agent` 測試：
  - backend alias parsing 與 serialized names。
  - private/local IP detection。
  - YAML bool/unquote parsing。
  - nested YAML config subset loading。
  - state 與 audit JSONL file round trips。
- 新增 `main.rs` helper 測試：
  - SQLite URL 轉 path。
  - readonly runtime directory remapping。
  - mail metadata/timestamp parsing。
  - archive raw/body size preference。
  - closest archived mail matching。
  - readonly overlay DB preparation。
  - CLI JSON report writing。
- 新增 `ai` response parser 測試：
  - OpenAI-compatible chat JSON。
  - SSE `data:` responses。
  - 簡易 `content` / `response` / `text` 相容格式。
  - response body summarization。
- 新增 `Config::load()` 測試：
  - 預設值。
  - runtime env parsing。
  - `READONLY_BLOCK_WRITES` 預設跟隨 `READONLY_MODE`。
  - remote debug 與 Cloudflare env 欄位。
- 新增 `web` helper 測試：
  - SQLite URL 轉 path。
  - overlay-relative DB path resolution。
  - 標準化 processing step JSON contract。
- 已安裝並執行 `cargo-llvm-cov`；backend line coverage 目前實測為 52.72%。
- 先前 P2 已新增手動重新處理測試：
  - 財務 rollback。
  - 規則重新命中。
  - 自動回覆草稿重建。
  - 透過指定 archive source path hydrate 郵件內容。

## 第一階段：補齊 Backend 安全缺口

目標：提高隱私、授權、remote-debug admin 行為的信心。

### 1. 設定持久化

- 驗證 `training_data_consent` 可正確寫入。
- 驗證 `training_consent_updated_at` 僅在 consent 變更時更新。
- 驗證其他 settings 更新不會重寫 consent timestamp。

### 2. 匯出閘道

- 驗證只有同意授權的使用者資料會被匯出。
- 驗證匯出內容一定經過脫敏。
- 驗證未授權角色不可呼叫匯出。

### 3. Admin Runtime 與 Remote Debug

- 驗證 runtime info 僅 admin 可讀。
- 驗證 developer/user 權限邊界。
- 驗證 runtime info 包含 configured DB path、active DB path、overlay path、readonly base、SSHFS posture、write API block status。

### 4. Support Package 隱私

- 驗證 support package preview 會遮罩或避免輸出 raw sensitive body content。
- 驗證 support package 包含 remote-debug posture 與 fallback metadata，但不 dump 原始郵件。
- 驗證非 admin 使用者不能請求其他使用者的 support package content。

## 第二階段：流程完整性

目標：覆蓋跨資料表狀態變化。

### 1. Chat Processing

- 驗證 chat completion 成功後會寫入 transcript。
- 驗證 onboarding step 邊界遞進。
- 驗證 chat feedback 會連到正確 transcript/user。

### 2. GDPR 刪除一致性

- 驗證刪除使用者時移除 `chat_transcripts`。
- 驗證刪除使用者時移除 `chat_feedback`。
- 驗證刪除使用者時移除該使用者的自動回覆草稿與財務紀錄。

### 3. 手動重新處理 Edge Cases

- 驗證 `force_reextract=false` 時，非 pending 郵件會被 skipped。
- 驗證 reprocess 會保留 `replied` 狀態。
- 驗證產生的草稿內容可透過 auto-reply APIs 讀取。

## 第三階段：前端單元測試

目標：覆蓋不一定會讓後端測試失敗的 UI state。

- Settings consent switch 顯示與 payload。
- Dashboard 多封郵件同時重新處理時，各自 timeline 獨立更新。
- Dashboard 自動回覆產生後的草稿查看/編輯內容。
- Finance analysis filters 與 empty states。

## 覆蓋率里程碑

- Milestone A：在 CI 恢復最新 `cargo llvm-cov` baseline。
- Milestone B：backend line coverage 25%，且隱私/授權測試完成。狀態：已達成。
- Milestone C：backend line coverage 35%，且 workflow integrity 測試完成。狀態：已達成。
- Milestone D：backend line coverage 50%，且 key web/mail/firewall handlers 已涵蓋。狀態：已達成，目前 52.72%。
- Milestone E：前端 Vitest coverage 啟用，涵蓋 settings 與 dashboard flows。

## 建議工具

Backend：

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --summary-only
```

Frontend：

```bash
npm run test -- --coverage
```

若前端測試工具尚未設定，先加入 Vitest 與 React Testing Library，再開始計算 frontend coverage。

## 備註

- 優先撰寫可重現、穩定的測試：純 helper、in-memory SQLite、本地 temp files、mock AI HTTP server。
- 法規/隱私關鍵路徑應明確納入測試保護。
- 避免依賴 live SMTP、Cloudflare、SSHFS 或外部 AI provider 的測試。
