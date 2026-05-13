# EC2 Production Checklist

Use this checklist before making an AI Mail Butler EC2 deployment the live MX target.

Related docs:

- [Running with nerdctl or Docker Compose](NERDCTL_COMPOSE_GUIDE.md)
- [Environment Examples](ENV-EXAMPLES.md)
- [EC2 Host Firewall Agent](EC2-HOST-FIREWALL-AGENT.md)
- [Cloudflare Cache Purge Token and Operations](CLOUDFLARE-CACHE-PURGE.md)
- [SSHFS Remote Debug Workflow](SSHFS-CLI-REMOTE-DEBUG.md)

## 1. Host and Data Layout

- [ ] EC2 instance has stable storage for the project directory.
- [ ] `docker compose` or `nerdctl compose` is installed.
- [ ] The deployment directory contains `docker-compose.yml`, `.env`, and optional override files.
- [ ] Persistent data is stored in `./ai-mail-butler-data`.
- [ ] Container DB path is `DATABASE_URL=sqlite:/app/data/data.sqlite`.
- [ ] Host DB path is `./ai-mail-butler-data/data.sqlite`.

## 2. DNS and Public URL

- [ ] `PUBLIC_URL` is set to the final HTTPS URL, for example `https://butler.example.com`.
- [ ] Web DNS record points to the EC2 host or load balancer.
- [ ] SMTP MX record points to the SMTP host name.
- [ ] SMTP host `A`/`AAAA` record is DNS-only when using Cloudflare.
- [ ] MX priority and target are verified with `dig MX example.com`.
- [ ] Port 25 inbound is allowed by the EC2 security group and by the instance firewall.

Cloudflare orange-cloud proxy does not support SMTP. Keep SMTP records DNS-only.

## 3. TLS and HTTP

- [ ] TLS terminates at a reverse proxy, load balancer, or another managed HTTPS layer.
- [ ] Reverse proxy forwards HTTP to the app `PORT`, usually `3000`.
- [ ] `PUBLIC_URL` uses `https://`, not `http://localhost`.
- [ ] Magic login links are tested from a real mailbox.

## 4. SMTP Receiving and Sending

- [ ] `SMTP_HOST_PORT=25` for production direct inbound SMTP.
- [ ] `ASSISTANT_EMAIL` uses the receiving domain or subdomain.
- [ ] `SMTP_RELAY_HOST`, `SMTP_RELAY_PORT`, `SMTP_RELAY_USER`, and `SMTP_RELAY_PASS` are set for outgoing mail.
- [ ] Gmail or M365 relay setup is tested with a magic login email.
- [ ] SPF/DKIM/DMARC records match the outgoing relay provider.

## 5. Host Firewall Agent

- [ ] App container does not run with `--privileged`, `NET_ADMIN`, or Docker socket access.
- [ ] Host firewall agent is installed as a systemd service.
- [ ] Agent socket exists at `/run/ai-mail-butler/firewall-agent.sock`.
- [ ] App container mounts `/run/ai-mail-butler:/run/ai-mail-butler`.
- [ ] `.env` enables:

```env
SMTP_SECURITY_BLOCKING_BACKEND=host-agent
SMTP_SECURITY_TEMP_BLOCK_ENABLED=true
SMTP_FIREWALL_AGENT_SOCKET=/run/ai-mail-butler/firewall-agent.sock
```

- [ ] `ai-mail-butler --mode fw --fw-action health` succeeds on the host.
- [ ] A short manual block/unblock test succeeds.

## 6. Cloudflare Cache Purge

- [ ] `CLOUDFLARE_ZONE_ID` is set if Admin Dashboard purge is needed.
- [ ] `CLOUDFLARE_API_TOKEN` has only Zone Cache Purge permission for the target zone.
- [ ] Admin/Developer user can purge a focused target after deployment.

## 7. Backup and Restore

- [ ] Back up `./ai-mail-butler-data/data.sqlite`.
- [ ] Back up `./ai-mail-butler-data/mail_spool`.
- [ ] Back up any production config outside git, especially `.env` and firewall agent config.
- [ ] Restore is tested on a non-production host.
- [ ] Backup job pauses writes or uses SQLite-safe backup tooling.

Minimum manual backup example:

```bash
mkdir -p backups
sqlite3 ai-mail-butler-data/data.sqlite ".backup 'backups/data-$(date +%Y%m%d-%H%M%S).sqlite'"
tar -C ai-mail-butler-data -czf "backups/mail-spool-$(date +%Y%m%d-%H%M%S).tar.gz" mail_spool
```

Minimum restore outline:

```bash
docker compose down
cp backups/data-YYYYmmdd-HHMMSS.sqlite ai-mail-butler-data/data.sqlite
tar -C ai-mail-butler-data -xzf backups/mail-spool-YYYYmmdd-HHMMSS.tar.gz
docker compose up -d
```

## 8. Remote Debug Posture

- [ ] Normal production has `REMOTE_DEBUG_SSHFS_ENABLED=false`.
- [ ] Remote debug overlay uses a separate `.env` posture and `docker-compose.sshfs.yml`.
- [ ] Remote debug mounts the whole data root, not only `mail_spool`, when DB inspection is needed.
- [ ] `READONLY_BLOCK_WRITES` is intentionally chosen for the debug session.

## 9. Final Smoke Test

```bash
docker compose ps
docker compose logs --tail=100
sqlite3 ai-mail-butler-data/data.sqlite "select count(*) from users;"
```

Then verify:

- [ ] Web UI opens at `PUBLIC_URL`.
- [ ] Magic link email is delivered.
- [ ] Inbound test mail reaches the Dashboard.
- [ ] Auto-reply draft can be viewed before sending.
- [ ] Firewall agent health is green if host blocking is enabled.
