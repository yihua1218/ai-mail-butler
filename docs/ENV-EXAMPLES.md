# Environment Examples

Use `.env.example` as the source of all supported keys. This file shows three practical profiles: production, staging, and remote-debug overlay.

Related docs:

- [Running with nerdctl or Docker Compose](NERDCTL_COMPOSE_GUIDE.md)
- [EC2 Production Checklist](EC2-PRODUCTION-CHECKLIST.md)
- [EC2 Host Firewall Agent](EC2-HOST-FIREWALL-AGENT.md)
- [SSHFS Remote Debug Workflow](SSHFS-CLI-REMOTE-DEBUG.md)
- [Cloudflare Cache Purge Token and Operations](CLOUDFLARE-CACHE-PURGE.md)

## Production

Production uses the compose data mount `/app/data`, public HTTPS URL, host port 25 for inbound SMTP, and host-agent blocking.

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

Staging should avoid claiming public SMTP unless intentionally testing MX delivery. Use a non-production URL and a non-port-25 SMTP host mapping.

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

Remote-debug overlay is for investigating synced or SSHFS-mounted production data without writing back to the production data root.

Use with:

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
```

Example:

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

Notes:

- `REMOTE_DEBUG_MODE=overlay` makes the entrypoint export `READONLY_MODE=true` and default `READONLY_BASE` to `REMOTE_DEBUG_MOUNT_POINT`.
- With Compose, set `OVERLAY_DIR` explicitly when you want to use `REMOTE_DEBUG_OVERLAY_DIR`; the compose file otherwise supplies `data/overlay`.
- Set `READONLY_BLOCK_WRITES=false` only when you intentionally want Dashboard write APIs to mutate the local overlay DB/files.
- Mount the remote data root, not just `mail_spool`, when you need DB-backed Dashboard debugging.
