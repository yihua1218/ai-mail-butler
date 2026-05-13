# AI Mail Butler: AWS EC2 Docker Deployment Guide

This guide reflects the current repository implementation. AI Mail Butler now ships with `Dockerfile`, `docker-compose.yml`, `.env.example`, an SSHFS override, SMTP security configuration, host firewall-agent support, and Cloudflare cache purge settings. Production deployments should use Docker Compose instead of hand-written Dockerfiles or one-off `docker run` commands.

## Current Implementation Map

| Area | Current implementation | Documentation update |
| --- | --- | --- |
| Container startup | `docker-compose.yml` builds or uses `${IMAGE_NAME}:${IMAGE_TAG}` | Compose is the primary deployment path |
| Data directory | host `./ai-mail-butler-data` mounts to `/app/data` | Treat the data root as the durable unit |
| Web port | `${PORT}:${PORT}` and container `PORT` are aligned | `.env` `PORT` must match reverse proxy expectations |
| SMTP port | `${SMTP_HOST_PORT}:25` | Use 2525 if host port 25 is unavailable |
| Remote debug | `docker-compose.sshfs.yml` + `docker-entrypoint.sh` support SSHFS | `REMOTE_DEBUG_MODE=overlay` enables readonly overlay DB |
| Write guard | `READONLY_MODE` and `READONLY_BLOCK_WRITES` | Overlay can allow local-overlay writes with `READONLY_BLOCK_WRITES=false` |
| SMTP security | app can call a host firewall agent socket | EC2 should prefer host-level enforcement |
| Cloudflare cache | Admin/Developer Dashboard can purge cache | `.env` needs zone ID and cache-purge-only token |

## 1. Install Docker

### Amazon Linux 2023

```bash
sudo dnf update -y
sudo dnf install -y docker git
sudo systemctl enable --now docker
sudo usermod -aG docker ec2-user
```

Reconnect SSH so the docker group applies.

### Ubuntu 22.04+

```bash
sudo apt update
sudo apt install -y docker.io docker-compose-plugin git
sudo systemctl enable --now docker
sudo usermod -aG docker ubuntu
```

Verify:

```bash
docker compose version
```

## 2. Clone the Project and Create Data Directory

```bash
git clone https://github.com/your-org/ai-mail-butler.git
cd ai-mail-butler
mkdir -p ai-mail-butler-data
cp .env.example .env
```

Compose mounts `./ai-mail-butler-data` to `/app/data`, so production data normally lives at:

```text
./ai-mail-butler-data/data.sqlite
./ai-mail-butler-data/mail_spool/
```

## 3. Configure `.env`

Minimum baseline:

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

Keep the remaining `.env.example` sections and enable them as needed:

- `READONLY_*` / `REMOTE_DEBUG_*`: remote data debugging and overlay.
- `SMTP_SECURITY_*`: SMTP abuse detection and blocking backend.
- `FIREWALL_AGENT_*`: EC2 host-level firewall agent.
- `CLOUDFLARE_*`: Dashboard cache purge.
- `M365_*`: Microsoft Graph / M365 mail sending.

Do not commit real `.env` secrets.

## 4. Start the Service

```bash
docker compose up -d --build
docker compose ps
docker compose logs -f ai-mail-butler
```

Update deployment:

```bash
git pull
docker compose up -d --build
```

Stop:

```bash
docker compose down
```

## 5. AWS Security Group and DNS

Open at least:

| Type | Port | Purpose |
| --- | --- | --- |
| HTTP | 80 | Reverse proxy or direct web access |
| HTTPS | 443 | Production Dashboard, Magic Link, and OAuth |
| SMTP | 25 | Receive forwarded mail |
| SSH | 22 | Server administration |

Cloudflare DNS notes:

- Web host may use Cloudflare proxy.
- SMTP/MX host such as `mail.example.com` must be DNS only.
- `PUBLIC_URL` must be a user-reachable HTTPS URL so Magic Links point to the correct host.

## 6. SMTP Security and Host Firewall Agent

The app's SMTP security layer detects suspicious behavior. For real EC2 IP blocking, run the firewall agent on the host:

1. Install `config/firewall-agent.yaml` to `/etc/ai-mail-butler/firewall-agent.yaml`.
2. Run the host service:

```bash
ai-mail-butler --mode firewall-agent --firewall-config /etc/ai-mail-butler/firewall-agent.yaml
```

3. Mount `/run/ai-mail-butler/firewall-agent.sock` into the app container.

See [EC2 Host Firewall Agent](docs/EC2-HOST-FIREWALL-AGENT.md).

## 7. Remote Debug / SSHFS Overlay

For normal production keep:

```bash
REMOTE_DEBUG_SSHFS_ENABLED=false
```

When you need in-container SSHFS remote-data debugging:

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
```

Typical overlay settings:

```bash
REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=overlay
REMOTE_DEBUG_REMOTE=ec2-user@prod:/home/ec2-user/ai-mail-butler/ai-mail-butler-data
REMOTE_DEBUG_MOUNT_POINT=/mnt/ai-mail-butler-data
READONLY_BLOCK_WRITES=false
```

`REMOTE_DEBUG_MODE=overlay` makes the entrypoint enable `READONLY_MODE=true` and defaults `READONLY_BASE` to the SSHFS mount point. The app copies remote `data.sqlite` into the overlay DB; later writes stay local to the overlay.

See [SSHFS Remote Debug Guide](docs/SSHFS-CLI-REMOTE-DEBUG.md).

## 8. Cloudflare Cache Purge

To purge cache from the Admin Dashboard:

```bash
CLOUDFLARE_ZONE_ID=your-zone-id
CLOUDFLARE_API_TOKEN=your-cache-purge-only-token
```

Use a token scoped only to Cache Purge for one zone. See [Cloudflare Cache Purge Token and Operations](docs/CLOUDFLARE-CACHE-PURGE.md).

## 9. Operations Checklist

After deployment:

```bash
docker compose ps
docker compose logs --tail=100 ai-mail-butler
```

Confirm:

- Dashboard opens through `PUBLIC_URL`.
- Magic Link URLs are correct.
- `ai-mail-butler-data/data.sqlite` persists.
- `ai-mail-butler-data/mail_spool` receives archives and processed mail.
- SMTP port 25 is reachable externally.
- Cloudflare MX host is DNS only.
- If host firewall agent is enabled, the container can access the socket.

## 10. Known Documentation Drift and Plan

Older docs described creating a Dockerfile manually and using `docker run`. The current baseline is:

1. `.env.example` is the configuration source.
2. `docker-compose.yml` is the production entrypoint.
3. `docker-compose.sshfs.yml` is only layered in for remote debugging.
4. The host firewall agent is the recommended EC2 SMTP blocking path.
5. Cloudflare purge, remote debug status, and selected operations are integrated into the Admin Dashboard.

See [Current Implementation and Execution Plan](docs/IMPLEMENTATION-EXECUTION-PLAN.md).
