# Current Implementation and Execution Plan

This document summarizes the current implementation, the drift from older documentation, and the next execution plan.

## Current Implementation Summary

### Deployment and Runtime

- The primary deployment entrypoint is `docker-compose.yml`.
- Host `./ai-mail-butler-data` mounts to container `/app/data`.
- `DATABASE_URL=sqlite:data/data.sqlite` maps to `/app/data/data.sqlite` in the container.
- `docker-compose.sshfs.yml` provides `/dev/fuse`, `SYS_ADMIN`, and SSH key mount for SSHFS.
- `docker-entrypoint.sh` mounts SSHFS when `REMOTE_DEBUG_SSHFS_ENABLED=true`.

### Remote Debug / Overlay

- `REMOTE_DEBUG_MODE=overlay` makes the entrypoint set `READONLY_MODE=true`.
- `READONLY_BASE` defaults to `REMOTE_DEBUG_MOUNT_POINT`.
- `prepare_readonly_overlay_db` copies the base DB into the overlay DB.
- A common overlay DB path is `data/overlay/data/data.sqlite`.
- `READONLY_BLOCK_WRITES` defaults to `READONLY_MODE`; set it to `false` to allow local overlay writes.

### Dashboard Email Content

- Dashboard API returns `preview`, `stored_content`, `plain_content`, and `html_content` from `emails`.
- If all content fields are empty, the backend searches `data/mail_spool/<user>/<message>/meta.txt` by user email, subject, and received time.
- The closest archived mail is rendered from `raw.eml`, with `body.txt` fallback.
- This fallback hydrates only the API response; it does not update the DB.

### Manual Reprocessing

- `/api/emails/process-manual` accepts multiple email IDs.
- Dashboard sends selected emails as parallel one-email requests so each row has its own process timeline.
- Backend manual reprocessing:
  - removes old unsent drafts,
  - rolls back and recalculates finance records,
  - reruns enabled email-rule matching,
  - regenerates auto-reply drafts when a rule matches,
  - uses archived mail fallback when DB content is empty.

### SMTP Security and Operations

- SMTP security observes suspicious AUTH probing by default.
- Real EC2 blocking should use the host firewall agent.
- `ai-mail-butler --mode firewall-agent` starts the host agent.
- `ai-mail-butler --mode fw` calls the agent through the Unix socket.

### Cloudflare Cache

- Admin/Developer Dashboard can purge focused cache targets or the whole zone.
- `.env` must set `CLOUDFLARE_ZONE_ID` and `CLOUDFLARE_API_TOKEN`.

## Main Drift from Older Docs

| Older documentation                             | Current implementation                                      | Update                                        |
|-------------------------------------------------|-------------------------------------------------------------|-----------------------------------------------|
| Manually create Dockerfile and run `docker run` | Repo provides Dockerfile and Compose                        | Docker guide is now Compose-first             |
| Remote path fixed to `/opt/ai-mail-butler/data` | Real deployments commonly use an `ai-mail-butler-data` root | Docs now describe the data-root principle     |
| Mount only spool                                | DB and spool both need to be visible                        | SSHFS guide now mounts the whole data root    |
| Overlay DB path not explained                   | `DATABASE_URL` + `OVERLAY_DIR` determine path               | Docs now show `data/overlay/data/data.sqlite` |
| No process for empty Dashboard content          | Backend has archived-mail fallback                          | Docs now include DB-length and archive checks |
| Manual reprocess described loosely              | It reruns finance, rules, and drafts                        | Docs now list the processing steps            |
| Blocking could be done inside container         | EC2 should use host firewall agent                          | Docker guide now favors host agent            |

## Execution Plan

### P0: Align Documentation with Reality

- [x] Update AWS Docker deployment guide to Compose-first.
- [x] Update SSHFS remote debug guide with overlay DB and archived-mail fallback.
- [x] Add this current-state and execution-plan document.
- [x] Link this document from README deployment sections.

### P1: Make Remote Debug Less Error-Prone

- [x] Show actual DB path, overlay DB path, and readonly base in Admin Runtime.
- [x] Mark email content source in Dashboard detail: DB, archived raw, or archived body.
- [x] Record archived-mail fallback hits in processing log details.
- [x] Add CLI REPL command `list-empty-archive` to list emails whose DB content is empty but archive content exists.

### P2: Stabilize Reprocessing

- [x] Return standardized processing step keys and translation keys from backend; Dashboard translates labels locally.
- [x] Add tests for finance rollback, rule match, draft rebuild, and archive fallback.
- [x] Strengthen duplicate-subject behavior by pinning reprocess requests to the archived source path already shown in Dashboard when available.
- [x] Allow Dashboard reprocess to pass the displayed archived raw/body path so the backend hydrates the DB row from that exact source.

### P3: Consolidate Deployment and Security Docs

- [ ] Update `NERDCTL_COMPOSE_GUIDE` image name/tag and port guidance.
- [ ] Add an EC2 production checklist covering DNS, TLS, SMTP, firewall agent, backup, and restore.
- [ ] Cross-link Cloudflare purge, host firewall agent, and remote debug docs.
- [ ] Split `.env` examples for production, staging, and remote-debug overlay.

### P4: Operations Automation

- [ ] Add `make doctor` or CLI doctor for DB schema, data root, mail spool, Cloudflare env, and firewall socket.
- [ ] Add backup/restore runbook for `data.sqlite`, `mail_spool`, and overlay.
- [ ] Include remote debug posture and fallback summary in support packages without leaking raw sensitive content.

## Validation

After each documentation or implementation change, run at least:

```bash
cargo check
npm run build
```

For reprocessing or content fallback changes, also inspect local synced data:

```bash
sqlite3 ai-mail-butler-data/overlay/data/data.sqlite \
  "select id, subject, status, length(coalesce(stored_content,'')) from emails order by received_at desc limit 10;"
```

For deployment documentation changes, verify names against `.env.example`, `docker-compose.yml`, `docker-entrypoint.sh`, and `src/config.rs`.
