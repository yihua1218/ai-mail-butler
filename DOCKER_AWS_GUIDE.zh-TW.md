# AI Mail Butler: AWS EC2 Docker 部署指南

本文件以目前程式庫實作為準，說明如何在 AWS EC2 上使用 Docker Compose 部署 AI Mail Butler。現行專案已內建 `Dockerfile`、`docker-compose.yml`、`.env.example`、SSHFS override、SMTP 安全設定與 Cloudflare cache purge 設定，不需要再手寫 Dockerfile 或用單一 `docker run` 管理正式服務。

## 目前實作對照

| 項目 | 目前實作 | 文件更新重點 |
| --- | --- | --- |
| 容器啟動 | `docker-compose.yml` 會 build 或使用 `${IMAGE_NAME}:${IMAGE_TAG}` | 以 Compose 為主，不再建議手動 `docker run` |
| 資料目錄 | host `./ai-mail-butler-data` 掛到容器 `/app/data` | 遠端同步與除錯時請以 data root 為單位 |
| Web port | `${PORT}:${PORT}`，容器內也使用 `PORT` | `.env` 的 `PORT` 需與反向代理一致 |
| SMTP port | `${SMTP_HOST_PORT}:25` | host port 25 不可用時可改 2525 |
| Remote debug | `docker-compose.sshfs.yml` + `docker-entrypoint.sh` 支援 SSHFS | `REMOTE_DEBUG_MODE=overlay` 會啟用 readonly overlay DB |
| 寫入保護 | `READONLY_MODE` 與 `READONLY_BLOCK_WRITES` | overlay 可搭配 `READONLY_BLOCK_WRITES=false` 允許寫入本地 overlay |
| SMTP 安全 | app 可呼叫 host firewall agent socket | EC2 建議用 host-level firewall agent，不讓容器持有 `NET_ADMIN` |
| Cloudflare cache | Admin/Developer Dashboard 可 purge | `.env` 需設定 zone id 與 cache-purge-only token |

## 1. 安裝 Docker

### Amazon Linux 2023

```bash
sudo dnf update -y
sudo dnf install -y docker git
sudo systemctl enable --now docker
sudo usermod -aG docker ec2-user
```

重新登入 SSH 讓 docker 群組生效。

### Ubuntu 22.04+

```bash
sudo apt update
sudo apt install -y docker.io docker-compose-plugin git
sudo systemctl enable --now docker
sudo usermod -aG docker ubuntu
```

重新登入 SSH 後確認：

```bash
docker compose version
```

## 2. 取得專案與資料目錄

```bash
git clone https://github.com/your-org/ai-mail-butler.git
cd ai-mail-butler
mkdir -p ai-mail-butler-data
cp .env.example .env
```

Compose 會將 `./ai-mail-butler-data` 掛載到容器內 `/app/data`。因此正式資料庫通常位於：

```text
./ai-mail-butler-data/data.sqlite
./ai-mail-butler-data/mail_spool/
```

## 3. 設定 `.env`

最小必要設定：

```bash
PORT=3000
HOST=0.0.0.0
PUBLIC_URL=https://butler.example.com
ADMIN_EMAIL=admin@example.com
DATABASE_URL=sqlite:data/data.sqlite
SMTP_HOST_PORT=25
ASSISTANT_EMAIL=assistant@mail.example.com

SMTP_RELAY_HOST=smtp.gmail.com
SMTP_RELAY_PORT=465
SMTP_RELAY_USER=your-email@gmail.com
SMTP_RELAY_PASS=your-app-password

AI_API_BASE_URL=https://your-ai-endpoint/v1
AI_API_KEY=your-api-key
AI_MODEL_NAME=your-model-name
```

建議保留 `.env.example` 中的其他區塊，依需要啟用：

- `READONLY_*` / `REMOTE_DEBUG_*`：遠端資料除錯與 overlay。
- `SMTP_SECURITY_*`：SMTP abuse 偵測與封鎖後端。
- `FIREWALL_AGENT_*`：EC2 host-level firewall agent。
- `CLOUDFLARE_*`：Dashboard cache purge。
- `M365_*`：Microsoft Graph / M365 寄信整合。

請勿將真實 `.env` commit 到 Git。

## 4. 啟動服務

```bash
docker compose up -d --build
docker compose ps
docker compose logs -f ai-mail-butler
```

更新部署：

```bash
git pull
docker compose up -d --build
```

停止：

```bash
docker compose down
```

## 5. AWS Security Group 與 DNS

至少開放：

| 類型 | Port | 說明 |
| --- | --- | --- |
| HTTP | 80 | 反向代理或直接 Web 存取 |
| HTTPS | 443 | 正式 Dashboard、Magic Link 與 OAuth 建議必備 |
| SMTP | 25 | 接收轉寄郵件 |
| SSH | 22 | 伺服器管理 |

Cloudflare DNS 重點：

- Web host 可走 Cloudflare proxy。
- SMTP/MX 指向的 `mail.example.com` 必須是 DNS only，不能開橘色雲朵。
- `PUBLIC_URL` 必須是使用者可開啟的 HTTPS URL，否則 Magic Link 會指錯地方。

## 6. SMTP 安全與 Host Firewall Agent

容器內的 SMTP security agent 會偵測可疑行為。正式 EC2 若要真的封鎖 IP，建議使用 host firewall agent：

1. 將 `config/firewall-agent.yaml` 安裝到 host，例如 `/etc/ai-mail-butler/firewall-agent.yaml`。
2. 在 host 以 systemd 執行：

```bash
ai-mail-butler --mode firewall-agent --firewall-config /etc/ai-mail-butler/firewall-agent.yaml
```

3. app container 透過 `/run/ai-mail-butler/firewall-agent.sock` 呼叫 agent。

詳見 [EC2 Host Firewall Agent](docs/EC2-HOST-FIREWALL-AGENT.md)。

## 7. Remote Debug / SSHFS Overlay

一般 production 請保持：

```bash
REMOTE_DEBUG_SSHFS_ENABLED=false
```

需要在容器啟動時掛載遠端 data root 時，使用 SSHFS override：

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
```

典型 overlay 設定：

```bash
REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=overlay
REMOTE_DEBUG_REMOTE=ec2-user@prod:/home/ec2-user/ai-mail-butler/ai-mail-butler-data
REMOTE_DEBUG_MOUNT_POINT=/mnt/ai-mail-butler-data
READONLY_BLOCK_WRITES=false
```

`REMOTE_DEBUG_MODE=overlay` 由 entrypoint 啟用 `READONLY_MODE=true`，並把 `READONLY_BASE` 指到 SSHFS 掛載點。程式會將遠端 `data.sqlite` 複製到 overlay DB，後續寫入留在 overlay，不直接修改遠端 DB。

更多細節請看 [SSHFS 遠端除錯指南](docs/SSHFS-CLI-REMOTE-DEBUG.zh-TW.md)。

## 8. Cloudflare Cache Purge

若要從 Admin Dashboard 清除快取，設定：

```bash
CLOUDFLARE_ZONE_ID=your-zone-id
CLOUDFLARE_API_TOKEN=your-cache-purge-only-token
```

API Token 應只授權單一 zone 的 Cache Purge 權限。詳見 [Cloudflare Cache Purge Token 與快取清除操作](docs/CLOUDFLARE-CACHE-PURGE.zh-TW.md)。

## 9. 營運檢查清單

部署後檢查：

```bash
docker compose ps
docker compose logs --tail=100 ai-mail-butler
```

確認：

- Dashboard 可透過 `PUBLIC_URL` 開啟。
- Magic Link 寄出後 URL 正確。
- `ai-mail-butler-data/data.sqlite` 持續存在。
- `ai-mail-butler-data/mail_spool` 有新信封存與 processed 記錄。
- SMTP port 25 可從外部連線。
- Cloudflare MX 是 DNS only。
- 若啟用 host firewall agent，socket 可被 container 存取。

## 10. 已知文件差異與後續計畫

舊版文件曾描述「如果沒有 Dockerfile 就手寫一份」與 `docker run` 單容器流程；目前實作已改為 Compose-first。新的部署基準如下：

1. `.env.example` 是設定來源。
2. `docker-compose.yml` 是正式啟動入口。
3. `docker-compose.sshfs.yml` 只在遠端除錯時疊加使用。
4. host firewall agent 是 EC2 SMTP 封鎖的建議路徑。
5. Cloudflare purge、remote debug 狀態與部分維運操作已整合到 Admin Dashboard。

新的執行計畫請見 [目前實作狀況與執行計畫](docs/IMPLEMENTATION-EXECUTION-PLAN.zh-TW.md)。
