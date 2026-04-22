# Readonly Overlay Mode

Readonly Overlay Mode lets you run AI Mail Butler against an existing data snapshot without modifying it. All writes are redirected to a separate overlay directory, while reads transparently fall back to the original (base) data when a file has not yet been written to the overlay. The SQLite database is copied once from the base snapshot into the overlay on first startup.

This is useful for:
- **Demo environments** — show real-looking data without risking mutations.
- **Staging / QA** — test new application versions against a production snapshot.
- **Read-only mirrors** — expose data to restricted users with no write capability.

---

## How It Works

### Three Layers of Protection

| Layer | What it does |
|---|---|
| **API write guard** | Middleware blocks all `POST`, `PUT`, and `DELETE` requests (except authentication endpoints) and returns `503 Service Unavailable` when readonly mode is active. |
| **File path remapping** | Every file write (SMTP spool, mail archive, attachments, decoded parts) is redirected to the overlay directory instead of the original logical path. |
| **Union read semantics** | File reads first check the overlay; if the requested file does not exist there, the request transparently falls back to the corresponding path inside `readonly_base`. Spool listings merge both directories (overlay entries take precedence, deduped by filename). |

### Database (SQLite)

On the first startup in readonly mode the application copies the base database file to `<overlay_dir>/data/data.sqlite`. All subsequent reads and writes use only this overlay copy, leaving the base snapshot untouched.

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|---|---|---|
| `READONLY_MODE` | `false` | Set to `true`, `1`, `yes`, or `on` to enable readonly overlay mode. |
| `READONLY_BASE` | _(empty)_ | Absolute or relative path to the base data snapshot directory. Falls back to base paths under this root for reads. |
| `OVERLAY_DIR` | `data/overlay` | Directory where all writes and the overlay DB copy are stored. Created automatically if it does not exist. |

### CLI Flags

```bash
cargo run -- \
  --readonly-mode \
  --readonly-base /path/to/production-snapshot \
  --overlay-dir /tmp/readonly-overlay
```

| Flag | Description |
|---|---|
| `--readonly-mode` | Enable readonly overlay mode (equivalent to `READONLY_MODE=true`). |
| `--readonly-base <path>` | Path to the base snapshot directory. |
| `--overlay-dir <path>` | Path to the overlay output directory. |

CLI flags override the corresponding environment variables when both are set.

---

## Example: Demo Mode

```bash
# Copy production snapshot to a read-only location
cp -r /var/lib/ai-mail-butler /srv/demo-snapshot

# Start the application in readonly mode
READONLY_MODE=true \
READONLY_BASE=/srv/demo-snapshot \
OVERLAY_DIR=/tmp/demo-overlay \
cargo run
```

On startup the application will:
1. Copy `/srv/demo-snapshot/data/data.sqlite` → `/tmp/demo-overlay/data/data.sqlite`.
2. Redirect all file writes to paths under `/tmp/demo-overlay/`.
3. Block all API write requests (returns `503`).
4. Serve reads from `/tmp/demo-overlay/` with transparent fallback to `/srv/demo-snapshot/`.

---

## Example: Docker Compose

```yaml
services:
  app:
    image: ai-mail-butler:latest
    environment:
      READONLY_MODE: "true"
      READONLY_BASE: "/data/base"
      OVERLAY_DIR: "/data/overlay"
    volumes:
      - ./production-snapshot:/data/base:ro
      - overlay-vol:/data/overlay
volumes:
  overlay-vol:
```

---

## Frontend Indicators

When readonly mode is active:

- A **yellow warning banner** is shown at the top of every page indicating that the instance is read-only and displaying the overlay and base paths.
- The **About** page shows the runtime mode (`Readonly Overlay` vs `Normal`).

---

## API

The `/api/about` endpoint includes readonly-mode metadata:

```json
{
  "readonly_mode_enabled": true,
  "readonly_base": "/srv/demo-snapshot",
  "overlay_dir": "/tmp/demo-overlay"
}
```

---

## Union Read Semantics (Detail)

### Spool File Listing

`union_list_eml_files` merges `.eml` files from both the overlay spool and the base spool:

1. All `.eml` files from `<overlay_dir>/data/mail_spool/` are collected first.
2. `.eml` files from `<readonly_base>/data/mail_spool/` whose filenames do **not** already appear in the overlay set are appended.
3. The final list is sorted by path.

This means:
- A file that exists in both locations uses the **overlay version** (the one that was written or modified locally).
- A file that exists only in the base is served read-only.

### File Read Fallback

`union_read_file` attempts to read from the resolved overlay path. If the file is not found there and readonly mode is active, it constructs the corresponding path under `readonly_base` and retries. This is used for reading spool `.eml` files during background processing.

### Directory Statistics (Data Deletion Snapshot)

`collect_dir_stats` is called with both the overlay sender directory and the corresponding base sender directory, so the file count and byte total reported in data-deletion snapshots reflect the union of both trees.

---

## Limitations

- Readonly mode protects **file I/O and API writes** at the application layer. It does not enforce OS-level filesystem permissions on the base directory (though mounting it read-only in Docker is recommended).
- The overlay DB copy is a full copy of the base DB. Schema migrations run on startup will modify the overlay DB copy, not the base.
- SMTP reception (port 25) is not blocked in readonly mode — incoming emails will be written to the overlay spool and processed normally. To block inbound SMTP you must configure firewall rules separately.
