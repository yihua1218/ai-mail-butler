# 使用 SSHFS + 本地 CLI 進行遠端 Mail Spool 除錯

本文件說明如何透過 SSHFS 掛載遠端 AI Mail Butler 伺服器的 spool 目錄，並使用你本地開發中的 CLI 工具，針對卡住或失敗的 `.eml` 郵件做除錯。

## 適用情境

適合以下情況：
- 伺服器的 `data/mail_spool` 有卡住未處理郵件
- 某些 `.eml` 一直處理失敗（例如 parse error、unknown sender）
- 想用本地最新程式碼除錯，不想先部署新版本到伺服器

## 前置需求

- 可 SSH 連線到遠端伺服器
- 本地已有 AI Mail Butler 專案與可執行 CLI 模式
- 本地 Rust 工具鏈（`cargo`）或已建好的本地執行檔
- 建議使用 SSH 金鑰登入

### 安裝 SSHFS

macOS：
- 安裝 macFUSE
- 安裝 SSHFS 客戶端（例如 `sshfs-mac`）

Linux：
- 透過套件管理器安裝 `sshfs`

## 掛載目錄建議

只掛載必要路徑，建議直接掛 spool：
- 遠端：`/opt/ai-mail-butler/data/mail_spool`
- 本地掛載點：`~/mnt/ai-mail-spool`

這樣可降低風險並提升效能。

## 1. 建立本地掛載點

```bash
mkdir -p ~/mnt/ai-mail-spool
```

## 2. 使用 SSHFS 掛載遠端 Spool

建議先用唯讀掛載，先觀察不修改：

```bash
sshfs devuser@your-server:/opt/ai-mail-butler/data/mail_spool \
  ~/mnt/ai-mail-spool \
  -o ro,reconnect,ServerAliveInterval=15,ServerAliveCountMax=3
```

如果需要重試流程而寫入檔案，再移除 `ro` 重新掛載。

## 3. 用本地 CLI 對掛載路徑做除錯

在本地專案根目錄執行：

單次處理：

```bash
cargo run -- --mode cli \
  --spool-dir ~/mnt/ai-mail-spool \
  --keep-files \
  --report-json ./data/cli-remote-report.json
```

互動 REPL 模式：

```bash
cargo run -- --mode cli --repl --spool-dir ~/mnt/ai-mail-spool --keep-files
```

REPL 常用指令：
- `list`
- `show <index|path>`
- `process <index|path>`
- `retry-unknown`
- `report`

## 4. 針對卡住/失敗信件的建議流程

建議步驟：
1. 用 `list` 找出待處理 `.eml`
2. 用 `show <index>` 檢查關鍵標頭（`From`、`To`、`Delivered-To`、`X-Original-To`）
3. 用 `process <index>` 觀察單封處理結果
4. 檢查 `--report-json` 報表中的 `parse_error`、`unknown_sender` 與統計

## 5. 對照遠端服務日誌

另開一個終端 SSH 到遠端：

```bash
ssh devuser@your-server
```

若使用 systemd，可查看：

```bash
journalctl -u ai-mail-butler -f
```

對照重點：
- 遠端服務錯誤訊息
- 本地 CLI 對同一封 `.eml` 的處理結果

## 6. 安全寫入流程（需要重試時）

如果一定要回補或搬移檔案：
1. 先卸載唯讀掛載
2. 重新用可寫模式掛載
3. 僅執行必要的目標操作
4. 操作完成後再切回唯讀

可避免誤改大量 production spool 檔案。

## 卸載

macOS / Linux：

```bash
umount ~/mnt/ai-mail-spool
```

若顯示 busy，先關閉正在使用該路徑的終端或編輯器再重試。

## 常見問題排查

### 掛載常斷線
- 使用 `reconnect,ServerAliveInterval=15,ServerAliveCountMax=3`
- 檢查網路品質與 SSH keepalive 設定

### 權限不足
- 確認遠端目錄權限與 SSH 帳號權限
- 先測試能否直接 SSH 存取該路徑

### CLI 看起來卡住
- 確認沒有誤用 `--watch`
- 留意大型檔案解析或網路檔案系統延遲
- 先在 REPL 用單封 `process` 逐步定位

### 與伺服器 worker 競爭同一批檔案
- 避免在同一路徑同時啟動 server spool worker 與可寫本地 CLI
- 建議先唯讀分析，再安排短時間可寫修復窗口

## 建議除錯模式

1. 先唯讀掛載
2. 本地 CLI 單次處理（`--keep-files` + JSON 報表）
3. 用 REPL 對單封深入分析
4. 必要時才短時間切可寫重試
5. 完成後卸載並整理結論
