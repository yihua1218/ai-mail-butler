# EC2 Production Checklist

將 AI Mail Butler EC2 部署切成正式 MX 目標前，請用這份 checklist 檢查。

相關文件：

- [使用 nerdctl 或 Docker Compose 執行](NERDCTL_COMPOSE_GUIDE.zh-TW.md)
- [環境變數範本](ENV-EXAMPLES.zh-TW.md)
- [EC2 host firewall agent](EC2-HOST-FIREWALL-AGENT.md)
- [Cloudflare cache purge token 與操作](CLOUDFLARE-CACHE-PURGE.zh-TW.md)
- [SSHFS remote debug workflow](SSHFS-CLI-REMOTE-DEBUG.zh-TW.md)

## 1. Host 與資料目錄

- [ ] EC2 instance 有穩定儲存空間放部署目錄。
- [ ] 已安裝 `docker compose` 或 `nerdctl compose`。
- [ ] 部署目錄包含 `docker-compose.yml`、`.env`，以及必要的 override 檔。
- [ ] 持久化資料放在 `./ai-mail-butler-data`。
- [ ] 容器內 DB path 是 `DATABASE_URL=sqlite:/app/data/data.sqlite`。
- [ ] Host 上 DB path 是 `./ai-mail-butler-data/data.sqlite`。

## 2. DNS 與公開網址

- [ ] `PUBLIC_URL` 設為正式 HTTPS URL，例如 `https://butler.example.com`。
- [ ] Web DNS record 指向 EC2 host 或 load balancer。
- [ ] SMTP MX record 指向 SMTP host name。
- [ ] 使用 Cloudflare 時，SMTP host 的 `A`/`AAAA` record 維持 DNS-only。
- [ ] 用 `dig MX example.com` 確認 MX priority 與 target。
- [ ] EC2 security group 與 instance firewall 都允許 inbound port 25。

Cloudflare orange-cloud proxy 不支援 SMTP。SMTP 相關 record 必須是 DNS-only。

## 3. TLS 與 HTTP

- [ ] TLS 由 reverse proxy、load balancer 或其他 HTTPS 層處理。
- [ ] Reverse proxy 將 HTTP 轉到 app `PORT`，通常是 `3000`。
- [ ] `PUBLIC_URL` 使用 `https://`，不是 `http://localhost`。
- [ ] 用真實信箱測過 magic login link。

## 4. SMTP 收信與寄信

- [ ] Production 直接收 SMTP 時設定 `SMTP_HOST_PORT=25`。
- [ ] `ASSISTANT_EMAIL` 使用收信 domain 或 subdomain。
- [ ] `SMTP_RELAY_HOST`、`SMTP_RELAY_PORT`、`SMTP_RELAY_USER`、`SMTP_RELAY_PASS` 已設定。
- [ ] Gmail 或 M365 relay 已用 magic login email 測過。
- [ ] SPF/DKIM/DMARC record 與 outgoing relay provider 一致。

## 5. Host Firewall Agent

- [ ] App container 沒有使用 `--privileged`、`NET_ADMIN` 或 Docker socket。
- [ ] Host firewall agent 已安裝為 systemd service。
- [ ] Agent socket 位於 `/run/ai-mail-butler/firewall-agent.sock`。
- [ ] App container 掛載 `/run/ai-mail-butler:/run/ai-mail-butler`。
- [ ] `.env` 啟用：

```env
SMTP_SECURITY_BLOCKING_BACKEND=host-agent
SMTP_SECURITY_TEMP_BLOCK_ENABLED=true
SMTP_FIREWALL_AGENT_SOCKET=/run/ai-mail-butler/firewall-agent.sock
```

- [ ] Host 上執行 `ai-mail-butler --mode fw --fw-action health` 成功。
- [ ] 短時間手動 block/unblock 測試成功。

## 6. Cloudflare Cache Purge

- [ ] 需要 Admin Dashboard purge 時已設定 `CLOUDFLARE_ZONE_ID`。
- [ ] `CLOUDFLARE_API_TOKEN` 只有目標 zone 的 Zone Cache Purge 權限。
- [ ] Admin/Developer 使用者可在部署後清除指定 target。

## 7. Backup 與 Restore

- [ ] 備份 `./ai-mail-butler-data/data.sqlite`。
- [ ] 備份 `./ai-mail-butler-data/mail_spool`。
- [ ] 備份 git 外的 production 設定，尤其 `.env` 與 firewall agent config。
- [ ] 已在非 production host 測過 restore。
- [ ] Backup job 會暫停寫入或使用 SQLite-safe backup。

最小手動 backup 範例：

```bash
mkdir -p backups
sqlite3 ai-mail-butler-data/data.sqlite ".backup 'backups/data-$(date +%Y%m%d-%H%M%S).sqlite'"
tar -C ai-mail-butler-data -czf "backups/mail-spool-$(date +%Y%m%d-%H%M%S).tar.gz" mail_spool
```

最小 restore 流程：

```bash
docker compose down
cp backups/data-YYYYmmdd-HHMMSS.sqlite ai-mail-butler-data/data.sqlite
tar -C ai-mail-butler-data -xzf backups/mail-spool-YYYYmmdd-HHMMSS.tar.gz
docker compose up -d
```

## 8. Remote Debug Posture

- [ ] 一般 production 維持 `REMOTE_DEBUG_SSHFS_ENABLED=false`。
- [ ] Remote debug overlay 使用獨立 `.env` posture 與 `docker-compose.sshfs.yml`。
- [ ] 需要查 DB 時，remote debug 掛載整個 data root，不只 `mail_spool`。
- [ ] Debug session 前明確選擇 `READONLY_BLOCK_WRITES`。

## 9. 最後 Smoke Test

```bash
docker compose ps
docker compose logs --tail=100
sqlite3 ai-mail-butler-data/data.sqlite "select count(*) from users;"
```

接著確認：

- [ ] Web UI 可從 `PUBLIC_URL` 開啟。
- [ ] Magic link email 有送達。
- [ ] Inbound test mail 會出現在 Dashboard。
- [ ] 自動回覆草稿送出前可以查看內容。
- [ ] 若啟用 host blocking，firewall agent health 是正常狀態。
