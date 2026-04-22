# Unit Test 覆蓋率計劃（第一階段）

## 目標
先補齊核心後端行為的單元測試，再擴展到法規與隱私敏感流程，以及前端關鍵互動。

## 目前基線
- 目前已有部分 `mail`、`web`、`services` 的輔助函式測試。
- 主要缺口仍在 API 授權、設定持久化、訓練資料匯出與脫敏流程。

### 目前覆蓋率快照（實測）
量測指令：
`LLVM_COV=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-cov LLVM_PROFDATA=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-profdata cargo llvm-cov --summary-only`

| 指標                   |   目前數值 |
| ---------------------- | ---------: |
| 日期                   | 2026-04-22 |
| 後端 line coverage     |     15.41% |
| 後端 function coverage |     23.20% |
| 後端 region coverage   |     15.80% |
| 前端 coverage          |   尚未量測 |

### 測試涵蓋範圍狀態
| 測試範圍                           | 狀態   | 說明                        |
| ---------------------------------- | ------ | --------------------------- |
| `first_email_address` 解析邏輯     | 已涵蓋 | 已有 `mail` helper 單元測試 |
| MIME 附件與 inline 文字分離        | 已涵蓋 | 已有 `mail` helper 單元測試 |
| 規則意圖判斷與去重插入             | 已涵蓋 | 已有 `web` 單元測試         |
| MX 解析 helper 行為                | 已涵蓋 | 已有 `web` 單元測試         |
| 訓練授權回答解析                   | 已涵蓋 | 已有 `web` 單元測試         |
| 訓練資料脫敏 regex 遮罩            | 已涵蓋 | 已有 `web` 單元測試         |
| Onboarding 問題遞進                | 已涵蓋 | 已有 `services` 單元測試    |
| consent 設定寫入與時間戳更新       | 未涵蓋 | 需補 DB 層測試              |
| 訓練匯出 API 授權邊界              | 未涵蓋 | 需補 API 權限測試           |
| 聊天成功後 transcript 寫入         | 未涵蓋 | 需補 API 流程測試           |
| GDPR 刪除時清除 `chat_transcripts` | 未涵蓋 | 需補交易刪除測試            |
| 前端 consent 開關互動與送出        | 未涵蓋 | 需補 Vitest/RTL 測試        |

## 第一階段（立即）
目標：先強化安全與正確性關鍵路徑。

### 1) 授權與 Onboarding
- 驗證首次 Onboarding 問題是否先詢問訓練授權。
- 驗證授權回答解析（英文 + 繁中）是否正確。
- 驗證回答不明確時的保守行為。

### 2) 脫敏處理
- 驗證以下資料會被正確遮罩：
  - Email
  - 美國/台灣電話格式
  - 長字串 token
- 驗證非敏感文字不會被過度破壞。

### 3) 設定持久化
- 驗證 `training_data_consent` 可正確寫入。
- 驗證 `training_consent_updated_at` 僅在值變更時更新。

### 4) 匯出閘道
- 驗證只有同意授權的使用者資料會被匯出。
- 驗證匯出內容一定經過脫敏。
- 驗證未授權角色不可呼叫匯出。

## 第二階段（下一步）
目標：流程完整性與 API 行為。

### 1) 聊天處理整合（單元 + 輕量整合）
- 驗證聊天完成後會寫入 transcript。
- 驗證 onboarding step 邊界遞進。

### 2) GDPR 刪除一致性
- 驗證刪除使用者時同步清除 `chat_transcripts` 與 feedback 紀錄。

### 3) 錯誤處理
- 驗證 GDPR 確認信失敗時，會保留詳細原因並正確記錄。

## 第三階段（後續）
目標：擴大品質防護範圍。

### 1) 前端單元測試
- Settings 授權開關顯示與送出 payload。
- Dashboard 回饋已讀/回覆狀態流轉。

### 2) 安全回歸測試
- 驗證 auth log token 已遮罩。
- 驗證訓練匯出 payload 不含原始個資。

## 建議覆蓋率里程碑
- 里程碑 A：整體 20%
- 里程碑 B：整體 35%
- 里程碑 C：整體 50% 以上（API 行為為主）

## 建議工具
- 後端覆蓋率：`cargo llvm-cov`
- 前端覆蓋率：`vitest --coverage`

## 備註
- 先優先撰寫可重現、可穩定執行的測試（純函式與查詢行為）。
- 法規/隱私關鍵路徑（授權閘道、脫敏、刪除）應優先納入測試保護。
