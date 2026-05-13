# Running with nerdctl or Docker Compose

This guide explains how to run AI Mail Butler with the repository's current `docker-compose.yml` using either `nerdctl compose` or `docker compose`.

The compose file already supports both source builds and GHCR images:

```yaml
image: ${IMAGE_NAME:-ghcr.io/yihua/ai-mail-butler}:${IMAGE_TAG:-latest}
build: .
```

If the image is available locally or remotely, Compose can use it. If you are developing from source, Compose can also build from the local `Dockerfile`.

## Prerequisites

- `nerdctl` plus `nerdctl-compose`, or Docker with the Compose plugin.
- A `.env` file based on `.env.example`.
- A host directory for persistent data. The default compose file mounts `./ai-mail-butler-data` to `/app/data`.

Related production docs:

- [EC2 Production Checklist](EC2-PRODUCTION-CHECKLIST.md)
- [Environment Examples](ENV-EXAMPLES.md)
- [EC2 Host Firewall Agent](EC2-HOST-FIREWALL-AGENT.md)
- [Cloudflare Cache Purge Token and Operations](CLOUDFLARE-CACHE-PURGE.md)
- [SSHFS Remote Debug Workflow](SSHFS-CLI-REMOTE-DEBUG.md)

## Configure `.env`

Start from the example:

```bash
cp .env.example .env
```

Minimum production-oriented values:

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

Important path rule:

- In Docker/nerdctl Compose, `DATABASE_URL` should normally be `sqlite:/app/data/data.sqlite`.
- On the host, the same DB file lives under `./ai-mail-butler-data/data.sqlite`.

## Image Name and Tag

The current default image is:

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=latest
```

For staging or production, prefer immutable tags when your CI publishes them:

```env
IMAGE_NAME=ghcr.io/yihua/ai-mail-butler
IMAGE_TAG=sha-<git-sha>
```

Pull explicitly when you want to verify image access before starting:

```bash
nerdctl pull ghcr.io/yihua/ai-mail-butler:latest
docker pull ghcr.io/yihua/ai-mail-butler:latest
```

## Port Guidance

The compose file maps:

```yaml
- "${PORT:-3000}:${PORT:-3000}"
- "${SMTP_HOST_PORT:-25}:25"
```

Use these common settings:

| Scenario | `PORT` | `SMTP_HOST_PORT` | Notes |
|---|---:|---:|---|
| Production MX target | `3000` | `25` | Needed for direct inbound SMTP from the internet. |
| Local or staging without public SMTP | `3000` | `2525` | Avoids host port 25 conflicts and privileged bind issues. |
| Behind reverse proxy | `3000` | `25` or `2525` | Proxy handles HTTP/TLS; SMTP still needs direct TCP routing. |

Cloudflare proxy does not proxy SMTP. DNS records for SMTP must be DNS-only.

## Start

With nerdctl:

```bash
nerdctl compose up -d
```

With Docker:

```bash
docker compose up -d
```

If you need SSHFS remote debug inside the container, add the SSHFS override:

```bash
nerdctl compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d
```

Keep SSHFS disabled for normal production operation.

## Verify

```bash
nerdctl compose ps
nerdctl compose logs --tail=100
```

Docker equivalent:

```bash
docker compose ps
docker compose logs --tail=100
```

Check the data directory:

```bash
ls -lah ai-mail-butler-data
```

You should see `data.sqlite` after the app initializes.

## Stop

```bash
nerdctl compose down
docker compose down
```

This removes the container but preserves `./ai-mail-butler-data`.
