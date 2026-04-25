# AI Mail Butler Chrome Extension (Local-Only Scaffold)

This folder contains a Chrome Extension Manifest V3 scaffold for local-first Gmail filtering and auto-reply workflows.

## Privacy Model

- Rules, settings, and audit metadata are stored in browser-local storage only.
- Email content is processed in the browser runtime and is not sent to this project server.
- No cloud sync is included in this scaffold.

## Files

- `manifest.json`: MV3 manifest and permissions.
- `background.js`: local rule engine, local audit, policy layer.
- `content-script.js`: Gmail thread extraction, visible-thread scan, draft insertion, guarded send trigger.
- `sidepanel.html` + `sidepanel.js` + `sidepanel.css`: local settings/rule management UI.

## Load Extension (Developer Mode)

1. Open Chrome and go to `chrome://extensions`.
2. Enable Developer Mode.
3. Click **Load unpacked** and select this `browser-extension/` folder.
4. Open Gmail (`https://mail.google.com`) and click the extension icon to open side panel.

## Operational Limits (Important)

This browser-local automation works only when:

1. The computer remains powered on.
2. Chrome remains open with Gmail available.
3. The browser/device does not sleep or hibernate.
4. Gmail tab state and extension workers are not suspended.

If any of these conditions break, unattended automation can pause or fail.

## Current Status

- Local rule filtering: implemented.
- Gmail visible-thread scan and highlight: implemented.
- Draft insertion and guarded send click flow: implemented.
- WebLLM integration: placeholder interface in `background.js`; replace `generateReplyWithLocalEngine` with a bundled local WebLLM runtime.

## Production Hardening Checklist

- Bundle WebLLM model runtime directly in extension package (no remote code).
- Add stricter selector health checks for Gmail UI changes.
- Add cooldown/idempotency per thread.
- Add encrypted local storage for sensitive rules and allowlists.
- Add fail-closed policy for ambiguous actions.
