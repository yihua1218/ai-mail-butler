# AI Mail Butler Chrome Extension（本地端版本骨架）

此資料夾提供一個 Chrome Extension Manifest V3 骨架，用於在瀏覽器本機執行 Gmail 規則過濾與自動回覆流程。

## 隱私模型

- 規則、設定與審計摘要僅儲存在瀏覽器本機。
- 郵件內容在瀏覽器端處理，不會傳到本專案伺服器。
- 此骨架不包含雲端同步。

## 主要檔案

- `manifest.json`：MV3 權限與入口設定。
- `background.js`：本地規則引擎、審計紀錄、策略判斷。
- `content-script.js`：讀取 Gmail thread、掃描列表、插入草稿、條件式寄送。
- `sidepanel.html` + `sidepanel.js` + `sidepanel.css`：本地規則與設定管理 UI。

## 安裝方式（開發人員模式）

1. 開啟 Chrome，前往 `chrome://extensions`。
2. 開啟「開發人員模式」。
3. 點「載入未封裝項目」，選取本資料夾 `browser-extension/`。
4. 開啟 Gmail（`https://mail.google.com`），點擊 extension 圖示開啟 side panel。

## 啟用範圍說明

- Gmail 自動化（內容讀取、清單掃描、草稿插入）只會在 `https://mail.google.com/*` 啟用。
- Side Panel 本身屬於 extension UI，仍可由工具列圖示開啟；但離開 Gmail 時不會有 Gmail 自動化能力。

## 權限與能力檢查

- Extension 啟動後，會先檢查必要 API 與權限（包含 `sidePanel`、`storage`、`scripting`、`alarms`、`activeTab`、`https://mail.google.com/*`）。
- 若有缺少或不可用項目，會在 side panel 的 `Permission & Capability Check` 顯示警示與限制。
- 若 `sidePanel` API 不可用，Extension 仍可啟動，但部分互動入口會受限。

## 錯誤訊息與下一步引導

- Side Panel 與 Gmail 內嵌面板的錯誤訊息會跟隨目前 extension 語系（英文 / 繁體中文）。
- 若語系同步失敗，會明確說明是「缺少目前分頁的頁面存取權限」或「目前分頁不支援同步」，並提示下一步。
- 若 Gmail 中出現 `Scan failed` / `Process failed` 類型問題，系統會優先提示是否為「Extension 剛重新載入，舊分頁 context 已失效」，並引導使用者重新整理 Gmail 分頁。
- Side Panel 也提供 `修復目前 Gmail 分頁`，可直接重新注入最新 content script 到目前 Gmail 分頁，適合處理 `sendMessage` 失效的情況。

## Side Panel 多語系

- Side Panel 支援英文與繁體中文。
- 語系設定儲存在 extension 的 `chrome.storage.local`（key: `extension_ui_lang`）。
- 目前不直接共用 Web 專案的 `localStorage`，避免跨來源與生命週期耦合。
- 可啟用 `Follow Web App Language`，從目前作用中分頁讀取網站 `localStorage.i18n_lang` 進行同步。
- 若目前作用中分頁不是你的 Web App，或該分頁沒有 `i18n_lang`，同步會提示失敗原因。
- Side Panel 內可設定 `Web App Origin 白名單`，只有白名單內的 origin 會被允許用來同步網站語系。
- 預設會先加入本機開發常用的 `http://localhost:5173` 與 `http://127.0.0.1:5173`，你也可以加入自己的正式站台 origin。

## 運作限制（務必先讀）

此本地端自動化僅在以下條件成立時可持續運作：

1. 電腦持續開機。
2. Chrome 持續開啟且 Gmail 可用。
3. 瀏覽器/電腦不可睡眠或休眠。
4. Gmail 分頁與 extension worker 未被系統暫停。

任一條件不成立時，無人值守自動化可能暫停或失敗。

## 目前狀態

- 本地規則過濾：已實作。
- Side Panel 規則新增 / 編輯：已實作，且表單說明支援英文與繁體中文。
- 語氣設定與允許動作的說明 / 選項：已支援英文與繁體中文。
- Gmail 可見清單掃描與高亮：已實作。
- Gmail 目前開啟信件的回覆草稿插入：已補強，會優先嘗試開啟該信件的回覆編輯框再插入草稿。
- WebLLM 實際推論整合：目前為 `background.js` 內本地 placeholder，後續需替換為 extension 內打包的 WebLLM runtime。

## 進入正式版前建議

- 將 WebLLM runtime 與模型打包進 extension（避免遠端程式碼）。
- 強化 Gmail DOM selector 健康檢查與 fail-closed 策略。
- 新增 thread 冪等與冷卻時間。
- 對敏感規則與 allowlist 使用本機加密儲存。
