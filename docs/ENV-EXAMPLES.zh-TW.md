# 環境變數範本

`.env.example` 是所有支援 key 的來源。本文件整理三種實務 profile：production、staging、remote-debug overlay。

相關文件：

- [使用 nerdctl 或 Docker Compose 執行](NERDCTL_COMPOSE_GUIDE.zh-TW.md)
- [EC2 Production Checklist](EC2-PRODUCTION-CHECKLIST.zh-TW.md)
- [EC2 Host Firewall Agent](EC2-HOST-FIREWALL-AGENT.md)
- [SSHFS Remote Debug Workflow](SSHFS-CLI-REMOTE-DEBUG.zh-TW.md)
- [Cloudflare Cache Purge Token 與操作](CLOUDFLARE-CACHE-PURGE.zh-TW.md)

## Production

Production 使用 compose data mount `/app/data`、公開 HTTPS URL、host port 25 收 SMTP，並透過 host-agent 執行封鎖。

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=latest

PORT=3000
HOST=0.0.0.0
SMTP_HOST_PORT=25
PUBLIC_URL=https://butler.example.com
RUST_LOG=info,ai_mail_butler=info
ADMIN_EMAIL=admin@example.com

DATABASE_URL=sqlite:/app/data/data.sqlite

READONLY_MODE=false
READONLY_BLOCK_WRITES=
READONLY_BASE=
OVERLAY_DIR=data/overlay
REMOTE_DEBUG_SSHFS_ENABLED=false

SMTP_RELAY_HOST=smtp.example.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=assistant@example.com
SMTP_RELAY_PASS=replace-me
ASSISTANT_EMAIL=assistant@mail.example.com

SMTP_SECURITY_CONFIG=config/smtp-security-agent.yaml
SMTP_SECURITY_ENABLED=true
SMTP_SECURITY_BLOCKING_BACKEND=host-agent
SMTP_SECURITY_TEMP_BLOCK_ENABLED=true
SMTP_FIREWALL_AGENT_SOCKET=/run/ai-mail-butler/firewall-agent.sock

AI_API_BASE_URL=https://api.openai.com/v1
AI_API_KEY=replace-me
AI_MODEL_NAME=replace-me

CLOUDFLARE_ZONE_ID=replace-me
CLOUDFLARE_API_TOKEN=replace-me
```

## Staging

Staging 除非刻意測 MX delivery，否則不要佔用公開 SMTP。建議使用非 production URL 與非 port 25 的 SMTP host mapping。

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=sha-<git-sha>

PORT=3000
HOST=0.0.0.0
SMTP_HOST_PORT=2525
PUBLIC_URL=https://staging-butler.example.com
RUST_LOG=info,ai_mail_butler=debug
ADMIN_EMAIL=admin@example.com

DATABASE_URL=sqlite:/app/data/data.sqlite

READONLY_MODE=false
READONLY_BLOCK_WRITES=
READONLY_BASE=
OVERLAY_DIR=data/overlay
REMOTE_DEBUG_SSHFS_ENABLED=false

SMTP_RELAY_HOST=smtp.example.com
SMTP_RELAY_PORT=587
SMTP_RELAY_USER=staging-assistant@example.com
SMTP_RELAY_PASS=replace-me
ASSISTANT_EMAIL=staging-assistant@mail.example.com

SMTP_SECURITY_ENABLED=true
SMTP_SECURITY_BLOCKING_BACKEND=disabled
SMTP_SECURITY_TEMP_BLOCK_ENABLED=false

AI_API_BASE_URL=https://api.openai.com/v1
AI_API_KEY=replace-me
AI_MODEL_NAME=replace-me

CLOUDFLARE_ZONE_ID=replace-me
CLOUDFLARE_API_TOKEN=replace-me
```

## Remote-Debug Overlay

Remote-debug overlay 用於檢查已同步或 SSHFS 掛載的 production data，同時避免寫回 production data root。

搭配以下指令：

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
```

範例：

```env
PORT=3000
HOST=0.0.0.0
SMTP_HOST_PORT=2525
PUBLIC_URL=http://localhost:3000
RUST_LOG=info,ai_mail_butler=debug
ADMIN_EMAIL=admin@example.com

DATABASE_URL=sqlite:data/data.sqlite

REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=overlay
REMOTE_DEBUG_ACCESS_MODE=readonly
REMOTE_DEBUG_REMOTE=ec2-user@example.com:/home/ec2-user/ai-mail-butler/ai-mail-butler-data
REMOTE_DEBUG_MOUNT_POINT=/mnt/ai-mail-butler-data
REMOTE_DEBUG_OVERLAY_DIR=/tmp/ai-mail-butler-overlay
REMOTE_DEBUG_SSHFS_OPTIONS=ro,reconnect,ServerAliveInterval=15,ServerAliveCountMax=3

READONLY_MODE=false
READONLY_BLOCK_WRITES=true
READONLY_BASE=
OVERLAY_DIR=/tmp/ai-mail-butler-overlay

SMTP_SECURITY_ENABLED=true
SMTP_SECURITY_BLOCKING_BACKEND=disabled
SMTP_SECURITY_TEMP_BLOCK_ENABLED=false

AI_API_BASE_URL=http://host.docker.internal:1234/v1
AI_API_KEY=
AI_MODEL_NAME=local-model
```

注意：

- `REMOTE_DEBUG_MODE=overlay` 會讓 entrypoint export `READONLY_MODE=true`，並讓 `READONLY_BASE` 預設指向 `REMOTE_DEBUG_MOUNT_POINT`。
- 使用 Compose 時，如果要使用 `REMOTE_DEBUG_OVERLAY_DIR` 指定的位置，請明確設定 `OVERLAY_DIR`；否則 compose 會提供預設值 `data/overlay`。
- 只有在你明確要讓 Dashboard write API 寫入本地 overlay DB/files 時，才設定 `READONLY_BLOCK_WRITES=false`。
- 需要用 Dashboard 查 DB 時，請掛載遠端 data root，而不是只掛 `mail_spool`。
