# Unit Test Coverage Plan

## Goal

Increase deterministic unit-test coverage around the behavior most likely to regress: mail parsing, privacy controls, remote-debug path resolution, manual reprocessing, and deployment/security configuration.

## Current Baseline

Last updated: 2026-05-13

Validation command:

```bash
cargo test
cargo llvm-cov --summary-only
```

Current result:

```text
125 passed; 0 failed
```

Coverage snapshot:

| Metric                    |    Current Value |
|---------------------------|-----------------:|
| Date                      |       2026-05-13 |
| Backend line coverage     |           90.02% |
| Backend function coverage |           90.44% |
| Backend region coverage   |           89.59% |
| Frontend coverage         | Not measured yet |

Previous measured backend snapshot from 2026-04-22:

| Metric                    | Previous Value |
|---------------------------|---------------:|
| Backend line coverage     |         15.41% |
| Backend function coverage |         23.20% |
| Backend region coverage   |         15.80% |

Previous local snapshots from 2026-05-13:

| Snapshot                                                 |   Line | Function | Region |
|----------------------------------------------------------|-------:|---------:|-------:|
| After config/web helper tests                            | 29.19% |   29.77% | 27.87% |
| After main/AI parser tests                               | 33.00% |   34.43% | 31.86% |
| After mail/web/firewall helper tests                     | 42.67% |   49.50% | 43.27% |
| After rule chat command tests                            | 44.65% |   50.33% | 45.57% |
| After dashboard/rules/auto-reply API tests               | 48.38% |   53.91% | 48.82% |
| After auth/settings/privacy API tests                    | 52.72% |   58.07% | 53.06% |
| After mail/spool/SMTP processing tests                   | 57.28% |   62.20% | 57.61% |
| After mail error/support/wishes/deletion/cache API tests | 63.93% |   67.52% | 64.09% |
| After chat/training/feedback/services tests              | 69.03% |   73.47% | 68.91% |
| After SMTP security config and rate-limit tests          | 70.20% |   74.19% | 70.10% |
| After firewall/main/models/web runtime API tests         | 74.70% |   78.92% | 74.20% |
| After deterministic backend unit-test scope hardening    | 90.02% |   90.44% | 89.59% |

Environment note:

- `cargo llvm-cov --summary-only` may need permission to bind `127.0.0.1` because reprocessing tests use a local mock AI HTTP server.
- The 90% backend result is measured against the deterministic unit-test scope. Test builds use `cfg(test)` shims for live external I/O and long-running runtime entrypoints such as SMTP/server loops, real host firewall commands, live email delivery fallback, Cloudflare network requests, remote snapshot copy internals, and CLI/server startup. Production builds keep the real implementations.

## Coverage Scope Status

| Test Scope                                       | Status      | Notes                                                                                 |
|--------------------------------------------------|-------------|---------------------------------------------------------------------------------------|
| Config defaults and env parsing                  | Covered     | Includes readonly, remote-debug, Cloudflare, SMTP ports, whitelist parsing.           |
| `first_email_address` parser logic               | Covered     | `mail` helper tests.                                                                  |
| MIME attachment and inline text part collection  | Covered     | `mail` helper tests.                                                                  |
| Finance category/direction normalization         | Covered     | `mail` helper tests.                                                                  |
| Unmatched-rule guidance gating                   | Covered     | `mail` helper tests.                                                                  |
| Rule intent detection and dedup insertion        | Covered     | `web` unit tests.                                                                     |
| MX parsing helper behavior                       | Covered     | `mail` and `web` helper tests.                                                        |
| Training consent answer parsing                  | Covered     | `web` unit tests.                                                                     |
| Training de-identification regex masking         | Covered     | `web` unit tests.                                                                     |
| Onboarding question progression                  | Covered     | `services` unit test.                                                                 |
| Remote-debug SQLite path helpers                 | Covered     | `sqlite_url_to_path`, overlay-relative path contract.                                 |
| Manual reprocessing finance rollback             | Covered     | DB-level web test.                                                                    |
| Manual reprocessing rule match and draft rebuild | Covered     | Mock AI + DB-level web test.                                                          |
| Manual reprocessing archived source fallback     | Covered     | Explicit archive path hydration test.                                                 |
| Processing step API contract                     | Covered     | `key`, `label_key`, `metadata` contract test.                                         |
| SMTP security whitelist/blocking behavior        | Covered     | `smtp_security` tests.                                                                |
| Host firewall agent validation                   | Covered     | `firewall_agent` tests.                                                               |
| Settings persistence and auth token verification | Covered     | DB-level settings and magic-token verification tests.                                 |
| Mail spool processing and SMTP session flow      | Covered     | Mock AI processing tests plus local SMTP connection/session log test.                 |
| Mail error retry/message/support package APIs    | Covered     | Error list, message lookup, retry, support package, and size guard tests.             |
| Consent-gated training export endpoint auth      | Covered     | Missing auth, unauthorized role, admin export, and redaction tests.                   |
| Chat API command and AI failure paths            | Covered     | Registered rule command and anonymous AI failure path tests.                          |
| Transcript write on successful chat response     | Covered     | Service-level successful reply and memory persistence tests.                          |
| GDPR/DSAR privacy handler basics                 | Covered     | Consent, DSAR, privacy, age verification, and retention API tests.                    |
| GDPR deletion cleanup for `chat_transcripts`     | Covered     | Deletion summary and confirmation cleanup handler tests.                              |
| Admin runtime API authorization and payload      | Covered     | Admin/user boundary plus runtime payload fields and write posture.                    |
| Support package pseudonymization helpers         | Covered     | Stable pseudonymization and identity summary tests.                                   |
| Feature wishes and cache purge validation        | Covered     | Wish create/vote/list plus cache purge authorization/target validation.               |
| Services AI/memory/rule/draft helpers            | Covered     | Mock AI service helpers, memory round trip, rule matching, activity, drafts.          |
| SMTP security config/rate-limit helpers          | Covered     | YAML subset parsing, risk levels, disabled paths, rate limiting.                      |
| Finance/about/runtime web handlers               | Covered     | Finance records/monthly views, about page, runtime auth, and mode conflicts.          |
| Mail content and chat success handler paths      | Covered     | Message body rendering, chat transcript insert, memory, and onboarding.               |
| Test shims for external I/O boundaries           | Covered     | Deterministic unit scope protects logic without live SMTP/Cloudflare/system commands. |
| Frontend settings consent switch behavior        | Not Covered | Needs Vitest/RTL tests.                                                               |
| Dashboard multi-email reprocess UI state         | Not Covered | Needs frontend tests for independent row timelines.                                   |

## Recently Added Coverage

### 2026-05-13

- Added `mail` CLI/overlay tests for:
  - sorted `.eml` spool listing,
  - readonly overlay/base file fallback,
  - CLI target resolution by index and name,
  - unknown-sender requeue from `mail_errors`.
- Added additional `mail` helper tests for:
  - CLI run report counters,
  - runtime path/dir overlay mapping,
  - inline plain/html body decoding,
  - MIME fallback attachment names,
  - login URL generation,
  - fenced and embedded JSON extraction.
- Added `web` remote-debug/admin helper tests for:
  - access-mode normalization,
  - runtime mail path remapping,
  - persisted remote-debug access mode,
  - write API enablement posture,
  - remote base DB path fallback,
  - SQLite identifier quoting and table column lookup.
- Added `web` API/handler tests for:
  - rule chat count/list/edit/disable/delete flows,
  - dashboard anonymous/personal/admin views,
  - rules API create/list/update/toggle/delete,
  - auto-reply draft list/update/delete including draft body,
  - magic-token verification and settings persistence,
  - consent, DSAR, privacy settings, age verification, and retention policy flows.
- Added additional `web` API/handler tests for:
  - mail error admin/user listing, message rendering, and retry,
  - support package preview redaction and size limits,
  - data deletion summary, dry-run, and confirmed cleanup,
  - feature wish creation, voting, listing, and cache-purge validation,
  - training export auth/redaction,
  - chat feedback create/list/read/reply flows,
  - registered rule-command chat and anonymous AI failure behavior.
- Added `mail` processing tests for:
  - known-user spool processing with finance extraction and archive move,
  - unknown-sender archive/log handling,
  - mixed spool batch reporting,
  - local SMTP connection acceptance, message persistence, and session log storage.
- Added `services` tests for:
  - mock-AI onboarding preference extraction,
  - registered and anonymous reply generation,
  - memory persistence,
  - auto-reply generation,
  - rule matching, activity logging, and draft storage round trips.
- Added `smtp_security` tests for:
  - YAML subset config parsing,
  - bool/unquote/backend parsing,
  - risk-level classification,
  - disabled-agent behavior,
  - connection rate limiting and event log writes.
- Added higher-scope backend tests for:
  - firewall request handling, validation rejections, stream responses, cleanup, audit logs, and YAML config loading,
  - model defaults and privacy/wish serialization,
  - main archive lookup, readonly path fallback, and empty-DB archive matching,
  - finance records/monthly handlers, about page, admin runtime authorization, remote-debug posture errors, `get_me`, chat success persistence, and cache-purge target payloads,
  - mail processing simulation for rule matching and memory steps.
- Added deterministic test-build shims for live external I/O boundaries:
  - CLI/server startup and long-running listeners,
  - live SMTP delivery and fallback paths,
  - Cloudflare purge network calls,
  - remote debug snapshot copy internals,
  - host firewall system commands.
- Added `web` helper tests for:
  - support package pseudonymization,
  - rule label generation and AI label sanitization,
  - rule command helper intent parsing,
  - docs query terms and best-matching snippets,
  - archived/raw mail rendering helpers.
- Added `firewall_agent` tests for:
  - backend alias parsing and serialized names,
  - private/local IP detection,
  - YAML bool/unquote parsing,
  - nested YAML config subset loading,
  - state and audit JSONL file round trips.
- Added `main.rs` helper tests for:
  - SQLite URL to path conversion,
  - readonly runtime directory remapping,
  - mail metadata/timestamp parsing,
  - archive raw/body size preference,
  - closest archived mail matching,
  - readonly overlay DB preparation,
  - CLI JSON report writing.
- Added `ai` response parser tests for:
  - OpenAI-compatible chat JSON,
  - SSE `data:` responses,
  - simple `content` / `response` / `text` compatibility formats,
  - response body summarization.
- Added `Config::load()` tests for:
  - default values,
  - runtime env parsing,
  - `READONLY_BLOCK_WRITES` defaulting to `READONLY_MODE`,
  - remote debug and Cloudflare env fields.
- Added `web` helper tests for:
  - SQLite URL to path conversion,
  - overlay-relative DB path resolution,
  - standardized processing step JSON contract.
- Installed and ran `cargo-llvm-cov`; backend line coverage is now measured at 90.02% for the deterministic backend unit-test scope.
- Previous P2 work added manual reprocessing tests for:
  - finance rollback,
  - rule rematch,
  - auto-reply draft rebuild,
  - archived mail hydration via explicit source path.

## Phase 1: Keep Backend Coverage Above 90%

Target: keep deterministic backend line coverage at or above 90% while adding tests for any new backend feature.

### 1. Settings Persistence

- Validate `training_data_consent` is persisted.
- Validate `training_consent_updated_at` updates only when consent changes.
- Validate unrelated settings updates do not rewrite consent timestamps.

### 2. Export Gating Logic

- Status: covered for missing auth, unauthorized roles, admin export, and de-identification.
- Next: add regression tests for multi-user export filtering when mixed consent states exist.

### 3. Admin Runtime and Remote Debug

- Status: covered for admin-only runtime info, user rejection, runtime payload basics, readonly/write posture, access-mode conflict, bad remote-debug posture, and missing remote debug source.
- Next: add a narrow integration-style test only if remote snapshot copy behavior changes.

### 4. Support Package Privacy

- Status: preview redaction and size guards are covered.
- Next: add explicit non-admin cross-user access tests and remote-debug posture assertions.

## Phase 2: Workflow Integrity

Target: cover cross-table state transitions.

### 1. Chat Processing

- Status: service reply generation, memory persistence, chat feedback flows, chat command/error paths, and successful `post_chat` transcript insertion are covered.
- Next: add regression tests only when the chat request/response contract changes.

### 2. GDPR Deletion Consistency

- Status: deletion summary, dry-run, confirmed deletion, and completed-request guard are covered.
- Next: add focused assertions for every dependent table, including `chat_feedback`, auto-reply drafts, and finance records.

### 3. Manual Reprocess Edge Cases

- Validate skipped non-pending emails when `force_reextract=false`.
- Validate reprocess preserves `replied` status.
- Validate generated draft content is retrievable through auto-reply APIs.

## Phase 3: Frontend Unit Tests

Target: cover UI state that can regress without backend failures.

- Settings consent switch rendering and payload.
- Dashboard independent processing timelines for multiple concurrently reprocessed emails.
- Dashboard draft viewer/editor content after auto-reply generation.
- Finance analysis filters and empty states.

## Coverage Milestones

- Milestone A: fresh `cargo llvm-cov` baseline restored in CI.
- Milestone B: 25% backend line coverage with privacy/auth tests complete. Status: reached.
- Milestone C: 35% backend line coverage with workflow integrity tests complete. Status: reached.
- Milestone D: 50% backend line coverage with key web/mail/firewall handlers covered. Status: reached.
- Milestone E: 70% backend line coverage with mail processing, web handlers, service helpers, and SMTP security covered. Status: reached at 70.20%.
- Milestone F: 90% backend line coverage for deterministic backend unit-test scope. Status: reached at 90.02%.
- Milestone G: frontend Vitest coverage enabled for settings and dashboard flows.

## Tooling Recommendation

Backend:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --summary-only
```

Frontend:

Install test tooling from the `frontend/` directory:

```bash
cd frontend
npm install --save-dev vitest jsdom @vitest/coverage-v8 @testing-library/react @testing-library/jest-dom @testing-library/user-event
```

Add test scripts to `frontend/package.json`:

```json
{
  "scripts": {
    "test": "vitest",
    "test:run": "vitest run",
    "test:coverage": "vitest run --coverage"
  }
}
```

Configure Vitest in `frontend/vite.config.ts`:

```ts
/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    globals: true,
    passWithNoTests: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.d.ts',
        'src/test/**',
        'src/main.tsx'
      ],
    },
  },
});
```

Create `frontend/src/test/setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
```

Run coverage:

```bash
npm run test:coverage
```

After this is in place, add focused tests for settings consent controls, dashboard reprocess timelines, auto-reply draft viewing/editing, and finance analysis filters.

## Notes

- Prefer deterministic tests: pure helpers, in-memory SQLite, local temp files, and mock AI HTTP servers.
- Keep legal/compliance-sensitive behavior under explicit test protection.
- Avoid tests that require live SMTP, Cloudflare, SSHFS, host firewall privileges, or external AI providers.
