# SSHFS Workflow: Remote Mail Spool Debugging with Local CLI

This guide explains how to mount a remote AI Mail Butler server spool directory via SSHFS and use your local development CLI to debug failed or stuck `.eml` processing.

## When to Use This

Use this workflow when:
- the server has mails stuck in `data/mail_spool`
- some `.eml` files repeatedly fail to parse or map to users
- you want to investigate with your latest local CLI code without deploying a new server build

## Prerequisites

- SSH access to the remote server
- Local AI Mail Butler repository with working CLI mode
- Local Rust toolchain (`cargo`) or a built local binary
- Recommended: SSH key authentication

### Install SSHFS

macOS:
- Install macFUSE
- Install SSHFS client (for example, `sshfs-mac`)

Linux:
- Install package `sshfs` from your distro

## Directory Strategy

Mount the remote data root so the local debug runtime can see both the database and spool:
- Remote: `/opt/ai-mail-butler/data`
- Local mount point: `~/mnt/ai-mail-butler-data`

This lets the app copy the remote `data.sqlite` into the local overlay database while reading remote files such as `mail_spool` through SSHFS.

## Container Startup Mount

When `REMOTE_DEBUG_SSHFS_ENABLED=true`, the container entrypoint mounts `REMOTE_DEBUG_REMOTE` at `REMOTE_DEBUG_MOUNT_POINT` before starting `ai-mail-butler`. The web app still only reports the configured posture; mount and unmount operations are not exposed through HTTP APIs.

```bash
REMOTE_DEBUG_SSHFS_ENABLED=true
REMOTE_DEBUG_MODE=overlay
REMOTE_DEBUG_ACCESS_MODE=readonly
REMOTE_DEBUG_REMOTE=devuser@your-server:/opt/ai-mail-butler/data
REMOTE_DEBUG_MOUNT_POINT=/mnt/ai-mail-butler-data
REMOTE_DEBUG_OVERLAY_DIR=/tmp/ai-mail-butler-overlay
```

Use `REMOTE_DEBUG_MODE=overlay` for full remote-data debugging. The entrypoint sets `READONLY_MODE=true` and defaults `READONLY_BASE` to the SSHFS mount point, so `data.sqlite` is copied into the local overlay before startup and file reads fall back to the mounted remote data root. Use `REMOTE_DEBUG_MODE=readonly` only when you want the mount indicator without enabling the app overlay workflow.
The Admin Dashboard also stores an admin-selected access posture (`readonly` by default, or `readwrite` for controlled retry windows). It reports intent only; the actual SSHFS mount/remount still happens outside the web app.

By default, `READONLY_MODE=true` also blocks write APIs. To keep overlay enabled while allowing writes into the local overlay database/files, set:

```bash
READONLY_BLOCK_WRITES=false
```

Docker and nerdctl need FUSE permissions for in-container SSHFS:

```bash
docker compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
nerdctl compose -f docker-compose.yml -f docker-compose.sshfs.yml up -d --build
```

Mounting also requires SSH credentials inside the container, for example by bind-mounting a read-only key or agent socket. Keep SSHFS disabled for normal production deployments.

## 1. Create Local Mount Point

```bash
mkdir -p ~/mnt/ai-mail-butler-data
```

## 2. Mount Remote Spool with SSHFS

Start with read-only mount for safe inspection:

```bash
sshfs devuser@your-server:/opt/ai-mail-butler/data \
  ~/mnt/ai-mail-butler-data \
  -o ro,reconnect,ServerAliveInterval=15,ServerAliveCountMax=3
```

If you need to perform retry operations that write files, remount without `ro`.

## 3. Run Local CLI Against Mounted Remote Spool

From your local repository root:

Single-pass debug run:

```bash
cargo run -- --mode cli \
  --spool-dir ~/mnt/ai-mail-butler-data/mail_spool \
  --keep-files \
  --report-json ./data/cli-remote-report.json
```

Interactive REPL debug:

```bash
cargo run -- --mode cli --repl --spool-dir ~/mnt/ai-mail-butler-data/mail_spool --keep-files
```

Useful REPL commands:
- `list`
- `show <index|path>`
- `process <index|path>`
- `retry-unknown`
- `report`

## 4. Investigate Stuck or Failing Files

Suggested sequence:
1. `list` to find pending `.eml` files
2. `show <index>` to inspect headers (`From`, `To`, `Delivered-To`, `X-Original-To`)
3. `process <index>` and capture result JSON
4. Check generated report (`--report-json`) for `parse_error`, `unknown_sender`, and counts

## 5. Cross-check with Remote Logs

Use SSH in a separate terminal:

```bash
ssh devuser@your-server
```

Then inspect service logs (example if running with systemd):

```bash
journalctl -u ai-mail-butler -f
```

Compare:
- remote runtime errors
- local CLI processing outcome on the same `.eml`

## 6. Safe Write Workflow (Retry Cases)

If you must requeue or move files:
1. Unmount read-only mount
2. Remount in read-write mode
3. Run targeted operations only
4. Switch back to read-only mode

This prevents accidental mass edits in production spool.

## Unmount

macOS / Linux:

```bash
umount ~/mnt/ai-mail-butler-data
```

If busy, close open terminals/editors using that path and retry.

## Troubleshooting

### Mount disconnects frequently
- Add `reconnect,ServerAliveInterval=15,ServerAliveCountMax=3`
- Verify SSH keepalive and network stability

### Permission denied
- Verify remote directory ownership and SSH user permissions
- Test direct SSH access first

### CLI appears to hang
- Ensure you are not in `--watch` mode when expecting single-pass
- Check large file parsing or network filesystem latency
- Try processing one file at a time in REPL

### Server and local CLI race conditions
- Avoid running server spool worker and write-capable local CLI at the same time on the same mounted path
- Prefer read-only analysis first, then controlled write window

## Recommended Debug Pattern

1. Read-only mount
2. Local CLI single-pass with `--keep-files` + JSON report
3. REPL one-file deep inspection
4. Controlled read-write retry only if needed
5. Unmount and document findings
