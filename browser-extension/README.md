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

## Build Downloadable Package

```bash
cd browser-extension
npm install
npm run package
```

The package task creates two outputs:

- `browser-extension/dist/ai-mail-butler-browser-extension-0.1.0/`: unpacked folder for Chrome / Edge Developer Mode.
- `frontend/public/downloads/ai-mail-butler-browser-extension-0.1.0.zip`: downloadable ZIP served by the WebLLM local page.

Chrome / Edge security rules do not allow a regular website to directly install an extension that is not published through the Chrome Web Store. Users should download the ZIP, unzip it, then load the unzipped folder through Developer Mode.

## Load Extension (Developer Mode)

Build the packaged WebLLM runtime once before loading the extension:

```bash
cd browser-extension
npm install
npm run build:webllm
```

1. Open Chrome and go to `chrome://extensions`.
2. Enable Developer Mode.
3. Click **Load unpacked** and select this `browser-extension/` folder.
4. Open Gmail (`https://mail.google.com`) and click the extension icon to open side panel.

## Web App Origin Whitelist

- The default whitelist includes the production web app origin `https://butler.yihua.app`.
- Local development origins `http://localhost:5173` and `http://127.0.0.1:5173` are included by default.
- The side panel lets users add or remove origins for web app language sync.

## Operational Limits (Important)

This browser-local automation works only when:

1. The computer remains powered on.
2. Chrome remains open with Gmail available.
3. The browser/device does not sleep or hibernate.
4. Gmail tab state and extension workers are not suspended.

If any of these conditions break, unattended automation can pause or fail.

## Current Status

- Local rule filtering: implemented.
- Gmail visible-thread scan and highlight: implemented for inbox/search-style visible result rows.
- Draft insertion and guarded send click flow: implemented with compose-box waiting.
- WebLLM integration: implemented through the packaged `vendor/web-llm/index.js` runtime built from `@mlc-ai/web-llm`. If the runtime is not built or WebGPU is unavailable, the extension reports that status and uses a deterministic local fallback.

## Production Hardening Checklist

- Bundle and review WebLLM runtime artifacts before Chrome Web Store submission.
- Add stricter selector health checks for Gmail UI changes.
- Add cooldown/idempotency per thread.
- Add encrypted local storage for sensitive rules and allowlists.
- Add fail-closed policy for ambiguous actions.
