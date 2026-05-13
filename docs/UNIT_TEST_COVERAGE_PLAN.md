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
48 passed; 0 failed
```

Coverage snapshot:

| Metric                    |    Current Value |
|---------------------------|-----------------:|
| Date                      |       2026-05-13 |
| Backend line coverage     |           29.19% |
| Backend function coverage |           29.77% |
| Backend region coverage   |           27.87% |
| Frontend coverage         | Not measured yet |

Previous measured backend snapshot from 2026-04-22:

| Metric                    | Previous Value |
|---------------------------|---------------:|
| Backend line coverage     |         15.41% |
| Backend function coverage |         23.20% |
| Backend region coverage   |         15.80% |

Environment note:

- `cargo llvm-cov --summary-only` may need permission to bind `127.0.0.1` because reprocessing tests use a local mock AI HTTP server.

## Coverage Scope Status

| Test Scope                                       | Status      | Notes                                                                       |
|--------------------------------------------------|-------------|-----------------------------------------------------------------------------|
| Config defaults and env parsing                  | Covered     | Includes readonly, remote-debug, Cloudflare, SMTP ports, whitelist parsing. |
| `first_email_address` parser logic               | Covered     | `mail` helper tests.                                                        |
| MIME attachment and inline text part collection  | Covered     | `mail` helper tests.                                                        |
| Finance category/direction normalization         | Covered     | `mail` helper tests.                                                        |
| Unmatched-rule guidance gating                   | Covered     | `mail` helper tests.                                                        |
| Rule intent detection and dedup insertion        | Covered     | `web` unit tests.                                                           |
| MX parsing helper behavior                       | Covered     | `mail` and `web` helper tests.                                              |
| Training consent answer parsing                  | Covered     | `web` unit tests.                                                           |
| Training de-identification regex masking         | Covered     | `web` unit tests.                                                           |
| Onboarding question progression                  | Covered     | `services` unit test.                                                       |
| Remote-debug SQLite path helpers                 | Covered     | `sqlite_url_to_path`, overlay-relative path contract.                       |
| Manual reprocessing finance rollback             | Covered     | DB-level web test.                                                          |
| Manual reprocessing rule match and draft rebuild | Covered     | Mock AI + DB-level web test.                                                |
| Manual reprocessing archived source fallback     | Covered     | Explicit archive path hydration test.                                       |
| Processing step API contract                     | Covered     | `key`, `label_key`, `metadata` contract test.                               |
| SMTP security whitelist/blocking behavior        | Covered     | `smtp_security` tests.                                                      |
| Host firewall agent validation                   | Covered     | `firewall_agent` tests.                                                     |
| Settings persistence for consent timestamps      | Not Covered | Needs DB-level tests.                                                       |
| Consent-gated training export endpoint auth      | Not Covered | Needs API authorization tests.                                              |
| Transcript write on successful chat response     | Not Covered | Needs API flow tests.                                                       |
| GDPR deletion cleanup for `chat_transcripts`     | Not Covered | Needs transaction/cleanup tests.                                            |
| Admin runtime API authorization and payload      | Not Covered | Important after remote-debug additions.                                     |
| Support package redaction and content limits     | Not Covered | Privacy-sensitive support workflow.                                         |
| Frontend settings consent switch behavior        | Not Covered | Needs Vitest/RTL tests.                                                     |
| Dashboard multi-email reprocess UI state         | Not Covered | Needs frontend tests for independent row timelines.                         |

## Recently Added Coverage

### 2026-05-13

- Added `Config::load()` tests for:
  - default values,
  - runtime env parsing,
  - `READONLY_BLOCK_WRITES` defaulting to `READONLY_MODE`,
  - remote debug and Cloudflare env fields.
- Added `web` helper tests for:
  - SQLite URL to path conversion,
  - overlay-relative DB path resolution,
  - standardized processing step JSON contract.
- Installed and ran `cargo-llvm-cov`; backend line coverage is now measured at 29.19%.
- Previous P2 work added manual reprocessing tests for:
  - finance rollback,
  - rule rematch,
  - auto-reply draft rebuild,
  - archived mail hydration via explicit source path.

## Phase 1: Finish Backend Safety Gaps

Target: raise confidence around privacy, auth, and remote-debug admin behavior.

### 1. Settings Persistence

- Validate `training_data_consent` is persisted.
- Validate `training_consent_updated_at` updates only when consent changes.
- Validate unrelated settings updates do not rewrite consent timestamps.

### 2. Export Gating Logic

- Validate export endpoint includes only users with consent enabled.
- Validate exported content is de-identified.
- Validate unauthorized roles are rejected.

### 3. Admin Runtime and Remote Debug

- Validate admin-only access for runtime info.
- Validate developer/user access boundaries where applicable.
- Validate runtime info includes configured DB path, active DB path, overlay path, readonly base, SSHFS posture, and write API block status.

### 4. Support Package Privacy

- Validate support package preview redacts or avoids raw sensitive body content.
- Validate support package includes remote-debug posture and fallback metadata without dumping raw mail.
- Validate non-admin users cannot request other users' support package content.

## Phase 2: Workflow Integrity

Target: cover cross-table state transitions.

### 1. Chat Processing

- Validate transcript insertion on successful chat completion.
- Validate onboarding-step progression boundaries.
- Validate chat feedback writes link to the correct transcript/user.

### 2. GDPR Deletion Consistency

- Validate user deletion removes `chat_transcripts`.
- Validate user deletion removes `chat_feedback`.
- Validate user deletion removes auto-reply drafts and finance records for that user.

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
- Milestone B: 25% backend line coverage with privacy/auth tests complete.
- Milestone C: 35% backend line coverage with workflow integrity tests complete.
- Milestone D: frontend Vitest coverage enabled for settings and dashboard flows.

## Tooling Recommendation

Backend:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --summary-only
```

Frontend:

```bash
npm run test -- --coverage
```

If frontend test tooling is not configured yet, add Vitest plus React Testing Library before counting frontend coverage.

## Notes

- Prefer deterministic tests: pure helpers, in-memory SQLite, local temp files, and mock AI HTTP servers.
- Keep legal/compliance-sensitive behavior under explicit test protection.
- Avoid tests that require live SMTP, Cloudflare, SSHFS, or external AI providers.
