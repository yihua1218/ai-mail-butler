# 使用 SSHFS + 本地 CLI 進行遠端 Mail Spool 除錯

本文件說明如何透過 SSHFS 掛載遠端 AI Mail Butler 伺服器的 spool 目錄，並使用你本地開發中的 CLI 工具，針對卡住或失敗的 `.eml` 郵件做除錯。

相關文件：

- [EC2 Production Checklist](EC2-PRODUCTION-CHECKLIST.zh-TW.md)
- [環境變數範本](ENV-EXAMPLES.zh-TW.md)
- [使用 nerdctl 或 Docker Compose 執行](NERDCTL_COMPOSE_GUIDE.zh-TW.md)
- [EC2 Host Firewall Agent](EC2-HOST-FIREWALL-AGENT.md)
- [Cloudflare Cache Purge Token 與操作](CLOUDFLARE-CACHE-PURGE.zh-TW.md)

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

建議掛載遠端 data root，讓本地除錯環境同時看到資料庫和 spool：
- 遠端：production 實際 data root，例如 `/home/ec2-user/ai-mail-butler/ai-mail-butler-data`
- 本地掛載點：`~/mnt/ai-mail-butler-data`

這樣 app 啟動時可以把遠端 `data.sqlite` 複製成 local overlay DB，同時透過 SSHFS 讀取遠端 `mail_spool` 等檔案。

目前 Docker Compose 的正式資料目錄是 host `./ai-mail-butler-data` 掛到容器 `/app/data`。若使用 `REMOTE_DEBUG_MODE=overlay`，程式啟動時會把 base DB 複製到 overlay DB；若 `DATABASE_URL=sqlite:data/data.sqlite` 且 `OVERLAY_DIR=data/overlay`，實際使用的 DB 會是：

```text
data/overlay/data/data.sqlite
```

如果你是在本地同步遠端資料除錯，請優先檢查：

```bash
sqlite3 ai-mail-butler-data/overlay/data/data.sqlite 'select count(*) from emails;'
```

不要只看 `data/data.sqlite` 或 `ai-mail-butler-data/data.sqlite`；那些可能是舊本地 DB 或空的 base DB。

## 容器啟動時自動掛載

當 `REMOTE_DEBUG_SSHFS_ENABLED=true` 時，容器 entrypoint 會在啟動 `ai-mail-butler` 前，先把 `REMOTE_DEBUG_REMOTE` 掛載到 `REMOTE_DEBUG_MOUNT_POINT`。Web App 仍然只顯示目前設定狀態；HTTP API 不提供 mount 或 umount 操作。

```bash
REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=overlay
REMOTE_DEBUG_REMOTE=devuser@your-server:/opt/ai-mail-butler/data
REMOTE_DEBUG_MOUNT_POINT=/mnt/ai-mail-butler-data
REMOTE_DEBUG_OVERLAY_DIR=/tmp/ai-mail-butler-overlay
OVERLAY_DIR=/tmp/ai-mail-butler-overlay
```

`REMOTE_DEBUG_MODE=overlay` 會由 entrypoint 強制啟用 `READONLY_MODE=true`，並在 `READONLY_BASE` 未設定時自動指向 SSHFS 掛載點。程式會先把遠端 `data.sqlite` 複製到本地 overlay DB，後續寫入留在本地 overlay，檔案讀取則可 fallback 到遠端 data root。
透過目前的 Compose 檔啟動時，如果希望 overlay 寫到 `REMOTE_DEBUG_OVERLAY_DIR`，請明確設定 `OVERLAY_DIR`；否則 Compose 會帶入預設 `data/overlay`。

目前實作還有一個 Dashboard 顯示 fallback：如果 `emails` 表某筆信件的 `preview`、`stored_content`、`plain_content`、`html_content` 全部為空，Dashboard API 會用使用者信箱、主旨與接收時間，到 `data/mail_spool/<user>/<message>/meta.txt` 找最接近的 archived mail，並解析 `raw.eml` 或 `body.txt` 回傳給前端顯示。這是為了處理遠端同步資料中「DB 有信件列，但內容欄位空；mail_spool 仍有原始信」的情況。

手動重新處理 `/api/emails/process-manual` 也會使用同樣 fallback，避免重新處理空內容信件時只拿空 `preview` 進行財務抽取與規則比對。

預設情況下，`READONLY_MODE=true` 也會封鎖 Web 寫入 API。如果要維持 overlay，但允許寫入本地 overlay DB/檔案，請設定：

```bash
READONLY_BLOCK_WRITES=false
```

Docker 和 nerdctl 需要額外 FUSE 權限，請搭配 SSHFS override 檔案：

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
nerdctl compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
```

容器內也必須有可用的 SSH 憑證，例如唯讀 bind mount SSH key 或 agent socket。一般 production 部署建議維持 `REMOTE_DEBUG_SSHFS_ENABLED=false`。

## Dashboard 環境狀態

Admin Dashboard 會顯示遠端除錯掛載的狀態，且只有 admin 能看到與調整。Web App 只記錄與顯示狀態，不會執行 `sshfs`、`mount` 或 `umount`。

```bash
REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=readonly
REMOTE_DEBUG_ACCESS_MODE=readonly
REMOTE_DEBUG_REMOTE=devuser@your-server:/opt/ai-mail-butler/data/mail_spool
REMOTE_DEBUG_MOUNT_POINT=~/mnt/ai-mail-spool
REMOTE_DEBUG_OVERLAY_DIR=/tmp/ai-mail-butler-overlay
```

`REMOTE_DEBUG_ACCESS_MODE` 預設為 `readonly`；admin 可在 Dashboard 暫時切成 `readwrite` 以標示受控的重試/修復窗口。實際 SSHFS 重新掛載仍需在 Web App 外完成。

## 1. 建立本地掛載點

```bash
mkdir -p ~/mnt/ai-mail-butler-data
```

## 2. 使用 SSHFS 掛載遠端 Data Root

建議先用唯讀掛載，先觀察不修改：

```bash
sshfs devuser@your-server:/opt/ai-mail-butler/data \
  ~/mnt/ai-mail-butler-data \
  -o ro,reconnect,ServerAliveInterval=15,ServerAliveCountMax=3
```

如果需要重試流程而寫入檔案，再移除 `ro` 重新掛載。

## 3. 用本地 CLI 對掛載路徑做除錯

在本地專案根目錄執行：

單次處理：

```bash
cargo run -- --mode cli \
  --spool-dir ~/mnt/ai-mail-butler-data/mail_spool \
  --keep-files \
  --report-json ./data/cli-remote-report.json
```

互動 REPL 模式：

```bash
cargo run -- --mode cli --repl --spool-dir ~/mnt/ai-mail-butler-data/mail_spool --keep-files
```

REPL 常用指令：
- `list`
- `show <index|path>`
- `process <index|path>`
- `retry-unknown`
- `list-empty-archive`
- `report`

## 4. 針對卡住/失敗信件的建議流程

建議步驟：
1. 用 `list` 找出待處理 `.eml`
2. 用 `show <index>` 檢查關鍵標頭（`From`、`To`、`Delivered-To`、`X-Original-To`）
3. 用 `process <index>` 觀察單封處理結果
4. 檢查 `--report-json` 報表中的 `parse_error`、`unknown_sender` 與統計

若 Dashboard 某封信看不到內容，請先查 overlay DB 內容長度：

```bash
sqlite3 ai-mail-butler-data/overlay/data/data.sqlite "
select id, subject, status,
       length(coalesce(preview,'')),
       length(coalesce(stored_content,'')),
       length(coalesce(plain_content,'')),
       length(coalesce(html_content,'')),
       received_at
from emails
where subject like '%關鍵字%'
order by received_at desc
limit 20;"
```

如果內容長度皆為 0，再檢查 spool archive：

```bash
find ai-mail-butler-data/overlay/data/mail_spool -type f -name meta.txt \
  -print | xargs rg -n "主旨關鍵字"
```

找到對應目錄後確認：

```bash
wc -c path/to/message/raw.eml path/to/message/body.txt
```

只要 `raw.eml` 或 `body.txt` 有內容，新的 Dashboard fallback 應可顯示該信。

也可以在 CLI REPL 中直接執行：

```text
list-empty-archive
```

此指令會列出 DB 內容欄位為空、但 archived `raw.eml` 或 `body.txt` 有內容的信件，輸出 email id、狀態、時間、archive 大小、主旨與來源路徑。

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
umount ~/mnt/ai-mail-butler-data
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

## 文件與實作差異整理

目前實作相較舊文件新增或修正：

- Compose-first 部署：`docker-compose.yml` 是主要入口，SSHFS 用 `docker-compose.sshfs.yml` 疊加。
- Overlay DB 實際位置依 `DATABASE_URL` 與 `OVERLAY_DIR` 組合決定，常見為 `data/overlay/data/data.sqlite`。
- Dashboard 會對內容空白的信件嘗試從 archived `raw.eml` / `body.txt` fallback 顯示。
- 手動重新處理會重新執行財務抽取、規則比對與草稿產生；若 DB 內容空，也會嘗試 archived mail fallback。
- Web App 只顯示與記錄 remote debug posture；實際 SSHFS mount/remount 仍在 entrypoint 或系統層處理。

後續執行計畫請見 [目前實作狀況與執行計畫](IMPLEMENTATION-EXECUTION-PLAN.zh-TW.md)。
