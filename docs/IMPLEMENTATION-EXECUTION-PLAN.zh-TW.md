# 目前實作狀況與執行計畫

本文件整理目前程式實作與既有文件之間的差異，並把後續要補齊的工作拆成可執行計畫。

## 實作現況摘要

### 部署與執行

- 主要部署入口是 `docker-compose.yml`。
- host `./ai-mail-butler-data` 會掛載到容器 `/app/data`。
- `DATABASE_URL=sqlite:data/data.sqlite` 在容器內對應 `/app/data/data.sqlite`。
- `docker-compose.sshfs.yml` 提供 SSHFS 所需的 `/dev/fuse`、`SYS_ADMIN` 與 SSH key mount。
- `docker-entrypoint.sh` 會在 `REMOTE_DEBUG_SSHFS_ENABLED=true` 時執行 SSHFS 掛載。

### Remote Debug / Overlay

- `REMOTE_DEBUG_MODE=overlay` 會在 entrypoint 中設定 `READONLY_MODE=true`。
- `READONLY_BASE` 預設會指向 `REMOTE_DEBUG_MOUNT_POINT`。
- `prepare_readonly_overlay_db` 會把 base DB 複製到 overlay DB。
- 常見 overlay DB 路徑為 `data/overlay/data/data.sqlite`。
- `READONLY_BLOCK_WRITES` 預設跟隨 `READONLY_MODE`；可設為 `false` 允許寫入本地 overlay。

### Dashboard 郵件內容

- Dashboard API 會從 `emails` 表回傳 `preview`、`stored_content`、`plain_content`、`html_content`。
- 若四個內容欄位皆空，後端會依使用者 email、subject、received_at 搜尋 `data/mail_spool/<user>/<message>/meta.txt`。
- 找到最接近的 archive 後，會解析 `raw.eml`，解析失敗或無內容時 fallback 到 `body.txt`。
- 此 fallback 只補回 API response，不會直接修改 DB。

### 手動重新處理

- `/api/emails/process-manual` 會支援多封 email id。
- Dashboard 前端會對多選郵件逐封並行呼叫 API，讓每封信都能顯示自己的處理流程。
- 後端手動重新處理會：
  - 清除舊的未寄草稿。
  - 清除並重新計算財務紀錄。
  - 重新比對啟用中的 email rules。
  - 命中規則時重新產生自動回覆草稿。
  - DB 內容空時使用 archived mail fallback 作為 source text。

### SMTP 安全與維運

- SMTP security 預設可觀察可疑 AUTH 探測。
- 真正封鎖 EC2 host 流量時，建議使用 host firewall agent。
- `ai-mail-butler --mode firewall-agent` 可啟動 host agent。
- `ai-mail-butler --mode fw` 可透過 Unix socket 呼叫 agent。

### Cloudflare Cache

- Admin/Developer Dashboard 可清除指定 cache target 或整站 cache。
- `.env` 需設定 `CLOUDFLARE_ZONE_ID` 與 `CLOUDFLARE_API_TOKEN`。

## 與舊文件的主要差異

| 舊文件描述                                  | 目前實作                                     | 更新結果                                          |
|---------------------------------------------|----------------------------------------------|---------------------------------------------------|
| 手寫 Dockerfile、用 `docker run` 啟動        | repo 已提供 Dockerfile 與 Compose            | Docker 指南改為 Compose-first                     |
| 遠端路徑範例固定 `/opt/ai-mail-butler/data` | 實際部署常用 `ai-mail-butler-data` data root | 文件改成 data root 原則與範例                     |
| 只說掛載 spool                              | 實作需要 DB 與 spool 同時可見                | SSHFS 文件改為掛載整個 data root                  |
| 未說明 overlay DB 實際位置                  | `DATABASE_URL` + `OVERLAY_DIR` 決定          | 文件加入 `data/overlay/data/data.sqlite` 檢查方式 |
| Dashboard 空內容無排查流程                  | 後端已有 archived mail fallback              | 文件加入查 DB 長度與查 archive 流程               |
| 手動重新處理只描述 AI 處理                  | 實作會重跑財務、規則、草稿                     | 文件加入處理步驟                                  |
| SMTP 封鎖可在容器內做                       | EC2 建議 host firewall agent                 | Docker 指南改成 host agent 為準                   |

## 新的實作執行計畫

### P0: 讓文件與現況一致

- [x] 更新 AWS Docker 部署指南，改為 Compose-first。
- [x] 更新 SSHFS remote debug 文件，補 overlay DB 與 archived mail fallback。
- [x] 建立本文件，集中整理現況、差異與計畫。
- [x] 將 README 的部署段落補上本文件連結。

### P1: 降低遠端除錯踩雷

- [ ] 在 Admin Runtime 區塊顯示目前實際 DB path、overlay DB path、readonly base。
- [ ] 在 Dashboard 郵件詳情中標示內容來源：DB、archived raw、archived body。
- [ ] 將 archived mail fallback 的命中結果寫入 processing log details，方便支援包檢查。
- [ ] 提供 CLI 指令列出「DB 內容空但 archive 有內容」的郵件。

### P2: 穩定重新處理流程

- [ ] 後端回傳更標準化的 processing step keys 與 localized labels，前端只負責翻譯。
- [ ] 針對手動重新處理補單元測試：財務 rollback、規則命中、草稿重建、archive fallback。
- [ ] 針對同主旨短時間重複郵件，加入更嚴格的 archive matching 條件，例如 message id 或 archive message key。
- [ ] 讓 Dashboard 可手動選擇「用哪個 archived raw mail 補這封 DB row」。

### P3: 部署與安全文件收斂

- [ ] 整理 `NERDCTL_COMPOSE_GUIDE`，修正 image name/tag 與 port 說明。
- [ ] 補一份 EC2 production checklist：DNS、TLS、SMTP、firewall agent、backup、restore。
- [ ] 將 Cloudflare purge、host firewall agent、remote debug 三份文件互相加上交叉連結。
- [ ] 明確區分 production、staging、remote-debug overlay 三種 `.env` 範本。

### P4: 維運自動化

- [ ] 提供 `make doctor` 或 CLI doctor 檢查：DB schema、data root、mail_spool、Cloudflare env、firewall socket。
- [ ] 提供資料備份/還原 runbook，涵蓋 `data.sqlite`、`mail_spool`、overlay。
- [ ] 在支援包中加入 remote debug posture 與內容 fallback 摘要，但避免洩漏原始敏感內容。

## 驗收方式

每次完成文件或實作項目後，至少確認：

```bash
cargo check
npm run build
```

涉及重新處理或資料 fallback 時，額外用本地同步資料查：

```bash
sqlite3 ai-mail-butler-data/overlay/data/data.sqlite \
  "select id, subject, status, length(coalesce(stored_content,'')) from emails order by received_at desc limit 10;"
```

涉及部署文件時，確認 `.env.example`、`docker-compose.yml`、`docker-entrypoint.sh`、`src/config.rs` 的設定名稱一致。
