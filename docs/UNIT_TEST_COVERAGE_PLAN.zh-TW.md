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
125 passed; 0 failed
```

Coverage 快照：

| 指標                      |   目前數值 |
|---------------------------|-----------:|
| 日期                      | 2026-05-13 |
| Backend line coverage     |     90.02% |
| Backend function coverage |     90.44% |
| Backend region coverage   |     89.59% |
| Frontend coverage         |   尚未量測 |

先前 2026-04-22 的 backend 實測 snapshot：

| 指標                      | 先前數值 |
|---------------------------|---------:|
| Backend line coverage     |   15.41% |
| Backend function coverage |   23.20% |
| Backend region coverage   |   15.80% |

2026-05-13 當天稍早的本地 snapshots：

| Snapshot                                                   |   Line | Function | Region |
|------------------------------------------------------------|-------:|---------:|-------:|
| 補完 config/web helper tests 後                            | 29.19% |   29.77% | 27.87% |
| 補完 main/AI parser tests 後                               | 33.00% |   34.43% | 31.86% |
| 補完 mail/web/firewall helper tests 後                     | 42.67% |   49.50% | 43.27% |
| 補完 rule chat command tests 後                            | 44.65% |   50.33% | 45.57% |
| 補完 dashboard/rules/auto-reply API tests 後               | 48.38% |   53.91% | 48.82% |
| 補完 auth/settings/privacy API tests 後                    | 52.72% |   58.07% | 53.06% |
| 補完 mail/spool/SMTP processing tests 後                   | 57.28% |   62.20% | 57.61% |
| 補完 mail error/support/wishes/deletion/cache API tests 後 | 63.93% |   67.52% | 64.09% |
| 補完 chat/training/feedback/services tests 後              | 69.03% |   73.47% | 68.91% |
| 補完 SMTP security config 與 rate-limit tests 後           | 70.20% |   74.19% | 70.10% |
| 補完 firewall/main/models/web runtime API tests 後         | 74.70% |   78.92% | 74.20% |
| 補完 deterministic backend unit-test scope hardening 後    | 90.02% |   90.44% | 89.59% |

環境備註：

- `cargo llvm-cov --summary-only` 可能需要允許 bind `127.0.0.1`，因為 reprocessing 測試會啟動本機 mock AI HTTP server。
- 90% backend 結果是以 deterministic unit-test scope 量測。Test build 針對 live external I/O 與長時間 runtime entrypoints 使用 `cfg(test)` shim，例如 SMTP/server loop、真實 host firewall 指令、live email delivery fallback、Cloudflare network request、remote snapshot copy internals，以及 CLI/server startup；production build 仍保留真實實作。

## 測試涵蓋範圍狀態

| 測試範圍                                      | 狀態   | 說明                                                                           |
|-----------------------------------------------|--------|--------------------------------------------------------------------------------|
| Config 預設值與 env parsing                   | 已涵蓋 | 包含 readonly、remote-debug、Cloudflare、SMTP port、whitelist parsing。             |
| `first_email_address` 解析邏輯                | 已涵蓋 | `mail` helper 測試。                                                            |
| MIME 附件與 inline 文字分離                   | 已涵蓋 | `mail` helper 測試。                                                            |
| 財務分類/方向 normalization                   | 已涵蓋 | `mail` helper 測試。                                                            |
| unmatched rule guidance gating                | 已涵蓋 | `mail` helper 測試。                                                            |
| 規則意圖判斷與去重插入                        | 已涵蓋 | `web` 單元測試。                                                                |
| MX 解析 helper 行為                           | 已涵蓋 | `mail` 與 `web` helper 測試。                                                   |
| 訓練授權回答解析                              | 已涵蓋 | `web` 單元測試。                                                                |
| 訓練資料脫敏 regex 遮罩                       | 已涵蓋 | `web` 單元測試。                                                                |
| Onboarding 問題遞進                           | 已涵蓋 | `services` 單元測試。                                                           |
| Remote-debug SQLite path helpers              | 已涵蓋 | `sqlite_url_to_path`、overlay-relative path contract。                           |
| 手動重新處理財務 rollback                     | 已涵蓋 | DB 層 web 測試。                                                                |
| 手動重新處理規則命中與草稿重建                | 已涵蓋 | Mock AI + DB 層 web 測試。                                                      |
| 手動重新處理 archived source fallback         | 已涵蓋 | 明確 archive path hydration 測試。                                              |
| Processing step API contract                  | 已涵蓋 | `key`、`label_key`、`metadata` contract 測試。                                    |
| SMTP security whitelist/blocking 行為         | 已涵蓋 | `smtp_security` 測試。                                                          |
| Host firewall agent validation                | 已涵蓋 | `firewall_agent` 測試。                                                         |
| settings 寫入與 magic-token 驗證              | 已涵蓋 | DB 層 settings 與 magic-token 驗證測試。                                        |
| 郵件 spool processing 與 SMTP session flow    | 已涵蓋 | Mock AI processing 測試與本機 SMTP connection/session log 測試。                |
| Mail error retry/message/support package APIs | 已涵蓋 | Error list、message lookup、retry、support package 與 size guard 測試。            |
| 訓練匯出 API 授權邊界                         | 已涵蓋 | Missing auth、unauthorized role、admin export 與 redaction 測試。                 |
| Chat API command 與 AI failure paths          | 已涵蓋 | Registered rule command 與 anonymous AI failure path 測試。                     |
| 聊天成功後 transcript 寫入                    | 已涵蓋 | Service 層 successful reply 與 memory persistence 測試。                        |
| GDPR/DSAR 隱私 handler 基本流程               | 已涵蓋 | Consent、DSAR、privacy、age verification、retention API 測試。                      |
| GDPR 刪除時清除 `chat_transcripts`            | 已涵蓋 | Deletion summary 與 confirmation cleanup handler 測試。                         |
| Admin runtime API 授權與 payload              | 已涵蓋 | Admin/user 邊界、runtime payload 欄位與 write posture。                          |
| Support package pseudonymization helpers      | 已涵蓋 | 穩定 pseudonymization 與 identity summary 測試。                                |
| Feature wishes 與 cache purge validation      | 已涵蓋 | Wish create/vote/list 與 cache purge authorization/target validation。          |
| Services AI/memory/rule/draft helpers         | 已涵蓋 | Mock AI service helpers、memory round trip、rule matching、activity、drafts。       |
| SMTP security config/rate-limit helpers       | 已涵蓋 | YAML subset parsing、risk levels、disabled paths、rate limiting。                  |
| Finance/about/runtime web handlers            | 已涵蓋 | Finance records/monthly views、about page、runtime auth 與 mode conflicts。       |
| 郵件內容與 chat success handler paths         | 已涵蓋 | Message body rendering、chat transcript insert、memory 與 onboarding。            |
| External I/O 邊界的 test shims                | 已涵蓋 | Deterministic unit scope 保護邏輯，不依賴 live SMTP/Cloudflare/system commands。 |
| 前端 consent 開關互動與送出                   | 未涵蓋 | 需補 Vitest/RTL 測試。                                                          |
| Dashboard 多封信重處理 UI state               | 未涵蓋 | 需補各 row 獨立 timeline 的前端測試。                                           |

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
- 新增更多 `web` API/handler 測試：
  - mail error admin/user listing、message rendering 與 retry。
  - support package preview redaction 與 size limits。
  - data deletion summary、dry-run 與 confirmed cleanup。
  - feature wish creation、voting、listing 與 cache-purge validation。
  - training export auth/redaction。
  - chat feedback create/list/read/reply flows。
  - registered rule-command chat 與 anonymous AI failure behavior。
- 新增 `mail` processing 測試：
  - known-user spool processing with finance extraction and archive move。
  - unknown-sender archive/log handling。
  - mixed spool batch reporting。
  - local SMTP connection acceptance、message persistence 與 session log storage。
- 新增 `services` 測試：
  - mock-AI onboarding preference extraction。
  - registered and anonymous reply generation。
  - memory persistence。
  - auto-reply generation。
  - rule matching、activity logging 與 draft storage round trips。
- 新增 `smtp_security` 測試：
  - YAML subset config parsing。
  - bool/unquote/backend parsing。
  - risk-level classification。
  - disabled-agent behavior。
  - connection rate limiting and event log writes。
- 新增更高範圍的 backend 測試：
  - firewall request handling、validation rejections、stream responses、cleanup、audit logs 與 YAML config loading。
  - model defaults 與 privacy/wish serialization。
  - main archive lookup、readonly path fallback 與 empty-DB archive matching。
  - finance records/monthly handlers、about page、admin runtime authorization、remote-debug posture errors、`get_me`、chat success persistence 與 cache-purge target payloads。
  - mail processing simulation for rule matching and memory steps。
- 新增 deterministic test-build shims，隔離 live external I/O 邊界：
  - CLI/server startup 與長時間 listeners。
  - live SMTP delivery 與 fallback paths。
  - Cloudflare purge network calls。
  - remote debug snapshot copy internals。
  - host firewall system commands。
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
- 已安裝並執行 `cargo-llvm-cov`；deterministic backend unit-test scope 的 backend line coverage 目前實測為 90.02%。
- 先前 P2 已新增手動重新處理測試：
  - 財務 rollback。
  - 規則重新命中。
  - 自動回覆草稿重建。
  - 透過指定 archive source path hydrate 郵件內容。

## 第一階段：維持 Backend Coverage 超過 90%

目標：維持 deterministic backend line coverage 大於等於 90%，並確保新增 backend feature 都同步補測試。

### 1. 設定持久化

- 驗證 `training_data_consent` 可正確寫入。
- 驗證 `training_consent_updated_at` 僅在 consent 變更時更新。
- 驗證其他 settings 更新不會重寫 consent timestamp。

### 2. 匯出閘道

- 狀態：missing auth、unauthorized role、admin export 與 de-identification 已涵蓋。
- 下一步：針對混合 consent 狀態的多使用者 export filtering 補 regression tests。

### 3. Admin Runtime 與 Remote Debug

- 狀態：admin-only runtime info、user rejection、runtime payload basics、readonly/write posture、access-mode conflict、bad remote-debug posture，以及 missing remote debug source 已涵蓋。
- 下一步：只有 remote snapshot copy 行為變更時，才補一個窄範圍 integration-style test。

### 4. Support Package 隱私

- 狀態：preview redaction 與 size guards 已涵蓋。
- 下一步：補明確的 non-admin cross-user access tests 與 remote-debug posture assertions。

## 第二階段：流程完整性

目標：覆蓋跨資料表狀態變化。

### 1. Chat Processing

- 狀態：service reply generation、memory persistence、chat feedback flows、chat command/error paths，以及 successful `post_chat` transcript insertion 已涵蓋。
- 下一步：只有 chat request/response contract 變更時再補 regression tests。

### 2. GDPR 刪除一致性

- 狀態：deletion summary、dry-run、confirmed deletion 與 completed-request guard 已涵蓋。
- 下一步：對每個 dependent table 補 focused assertions，包含 `chat_feedback`、auto-reply drafts 與 finance records。

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
- Milestone D：backend line coverage 50%，且 key web/mail/firewall handlers 已涵蓋。狀態：已達成。
- Milestone E：backend line coverage 70%，且 mail processing、web handlers、service helpers 與 SMTP security 已涵蓋。狀態：已達成，目前 70.20%。
- Milestone F：deterministic backend unit-test scope 的 backend line coverage 90%。狀態：已達成，目前 90.02%。
- Milestone G：前端 Vitest coverage 啟用，涵蓋 settings 與 dashboard flows。

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
- 避免依賴 live SMTP、Cloudflare、SSHFS、host firewall 權限或外部 AI provider 的測試。
