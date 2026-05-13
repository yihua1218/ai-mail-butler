# 使用 nerdctl 或 Docker Compose 執行

本指南說明如何用目前 repo 內的 `docker-compose.yml`，透過 `nerdctl compose` 或 `docker compose` 啟動 AI Mail Butler。

目前 compose 檔已同時支援 GHCR image 與本機原始碼 build：

```yaml
image: ${IMAGE_NAME:-ghcr.io/yihua/ai-mail-butler}:${IMAGE_TAG:-latest}
build: .
```

若 image 已存在於本機或 registry，Compose 可以使用 image；若你正在開發原始碼，也可以用本機 `Dockerfile` build。

## 前置需求

- `nerdctl` 與 `nerdctl-compose`，或 Docker Compose plugin。
- 依 `.env.example` 建立 `.env`。
- 準備持久化資料目錄。預設 compose 會把 host `./ai-mail-butler-data` 掛到容器 `/app/data`。

相關 production 文件：

- [EC2 production checklist](EC2-PRODUCTION-CHECKLIST.zh-TW.md)
- [環境變數範本](ENV-EXAMPLES.zh-TW.md)
- [EC2 host firewall agent](EC2-HOST-FIREWALL-AGENT.md)
- [Cloudflare cache purge token 與操作](CLOUDFLARE-CACHE-PURGE.zh-TW.md)
- [SSHFS remote debug workflow](SSHFS-CLI-REMOTE-DEBUG.zh-TW.md)

## 設定 `.env`

先複製範本：

```bash
cp .env.example .env
```

Production 常用的最小設定：

```env
PORT=3000
HOST=0.0.0.0
SMTP_HOST_PORT=25
PUBLIC_URL=https://butler.example.com
ADMIN_EMAIL=admin@example.com

DATABASE_URL=sqlite:/app/data/data.sqlite

SMTP_RELAY_HOST=smtp.example.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=assistant@example.com
SMTP_RELAY_PASS=your-smtp-secret
ASSISTANT_EMAIL=assistant@mail.example.com

AI_API_BASE_URL=https://api.openai.com/v1
AI_API_KEY=your-ai-api-key
AI_MODEL_NAME=your-model-name
```

重要路徑規則：

- Docker/nerdctl Compose 內，`DATABASE_URL` 通常應該是 `sqlite:/app/data/data.sqlite`。
- 在 host 上，同一個 DB 檔案位於 `./ai-mail-butler-data/data.sqlite`。

## Image 名稱與 Tag

目前預設 image 是：

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=latest
```

Staging 或 production 建議使用 CI 發出的不可變 tag：

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=sha-<git-sha>
```

若要先確認 registry 權限，可手動 pull：

```bash
nerdctl pull ghcr.io/yihua/ai-mail-butler:latest
docker pull ghcr.io/yihua/ai-mail-butler:latest
```

## Port 說明

Compose 目前映射：

```yaml
- "${PORT:-3000}:${PORT:-3000}"
- "${SMTP_HOST_PORT:-25}:25"
```

常見設定：

| 情境 | `PORT` | `SMTP_HOST_PORT` | 說明 |
|---|---:|---:|---|
| Production MX 目標 | `3000` | `25` | 網際網路直接投遞 SMTP 時需要 host port 25。 |
| 本機或不公開 SMTP 的 staging | `3000` | `2525` | 避免 host port 25 權限或衝突問題。 |
| HTTP/TLS 走 reverse proxy | `3000` | `25` 或 `2525` | Proxy 處理 HTTP/TLS；SMTP 仍需直接 TCP routing。 |

Cloudflare proxy 不代理 SMTP。SMTP 相關 DNS record 必須維持 DNS-only。

## 啟動

使用 nerdctl：

```bash
nerdctl compose up -d
```

使用 Docker：

```bash
docker compose up -d
```

若需要在容器內使用 SSHFS remote debug，疊加 SSHFS override：

```bash
nerdctl compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
```

一般 production 運行請保持 SSHFS 關閉。

## 驗證

```bash
nerdctl compose ps
nerdctl compose logs --tail=100
```

Docker 對應指令：

```bash
docker compose ps
docker compose logs --tail=100
```

檢查資料目錄：

```bash
ls -lah ai-mail-butler-data
```

App 初始化後應該會看到 `data.sqlite`。

## 停止

```bash
nerdctl compose down
docker compose down
```

這會移除容器，但保留 `./ai-mail-butler-data`。
