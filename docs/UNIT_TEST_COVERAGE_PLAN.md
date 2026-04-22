# Unit Test Coverage Plan (Phase 1)

## Goal
Increase unit-test coverage for core backend behavior first, then expand to compliance-sensitive and frontend-critical flows.

## Current Baseline
- Existing tests cover selected helper functions in `mail`, `web`, and `services`.
- Major gaps remain in API authorization, settings persistence, transcript export, and privacy/de-identification workflows.

### Coverage Snapshot (Measured)
Measurement command:
`LLVM_COV=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-cov LLVM_PROFDATA=/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/llvm-profdata cargo llvm-cov --summary-only`

| Metric                    |    Current Value |
| ------------------------- | ---------------: |
| Date                      |       2026-04-22 |
| Backend line coverage     |           15.41% |
| Backend function coverage |           23.20% |
| Backend region coverage   |           15.80% |
| Frontend coverage         | Not measured yet |

### Coverage Scope Status
| Test Scope                                      | Status      | Notes                           |
| ----------------------------------------------- | ----------- | ------------------------------- |
| `first_email_address` parser logic              | Covered     | `mail` helper unit test exists  |
| MIME attachment and inline text part collection | Covered     | `mail` helper unit test exists  |
| Rule intent detection and dedup insertion       | Covered     | `web` unit tests exist          |
| MX parsing helper behavior                      | Covered     | `web` unit tests exist          |
| Training consent answer parsing                 | Covered     | `web` unit tests exist          |
| Training de-identification regex masking        | Covered     | `web` unit tests exist          |
| Onboarding question progression                 | Covered     | `services` unit test exists     |
| Settings persistence for consent timestamps     | Not Covered | Needs DB-level tests            |
| Consent-gated training export endpoint auth     | Not Covered | Needs API authorization tests   |
| Transcript write on successful chat response    | Not Covered | Needs API flow tests            |
| GDPR deletion cleanup for `chat_transcripts`    | Not Covered | Needs transaction/cleanup tests |
| Frontend settings consent switch behavior       | Not Covered | Needs Vitest/RTL tests          |

## Phase 1 (Immediate)
Target: strengthen core safety and correctness paths.

### 1) Consent and Onboarding
- Validate onboarding first-question sequence includes training-consent prompt.
- Validate yes/no parsing for consent answers (EN + zh-TW expressions).
- Validate fallback behavior when answer is ambiguous.

### 2) Data De-identification
- Validate redaction of:
  - email addresses
  - US/TW phone patterns
  - long token-like strings
- Validate non-sensitive text remains readable.

### 3) Settings Persistence
- Validate `training_data_consent` is correctly persisted.
- Validate `training_consent_updated_at` updates only when consent value changes.

### 4) Export Gating Logic
- Validate export endpoint includes only users with consent enabled.
- Validate exported content is de-identified.
- Validate unauthorized roles are rejected.

## Phase 2 (Next)
Target: API and workflow integrity.

### 1) Chat Processing Integration (Unit + lightweight integration)
- Validate transcript insertion on chat completion.
- Validate onboarding-step progression boundaries.

### 2) GDPR Deletion Consistency
- Validate user deletion also removes `chat_transcripts` and feedback records.

### 3) Error Handling
- Validate GDPR email send failure captures detailed reason and logs appropriately.

## Phase 3 (Future)
Target: broader quality envelope.

### 1) Frontend Unit Tests
- Settings consent switch rendering and payload.
- Dashboard feedback/read/reply state transitions.

### 2) Security Regression Tests
- Ensure token masking in auth logs.
- Ensure no raw PII appears in training export payload.

## Suggested Coverage Milestones
- Milestone A: 20% overall
- Milestone B: 35% overall
- Milestone C: 50%+ overall with API behavior focus

## Tooling Recommendation
- Backend coverage: `cargo llvm-cov`
- Frontend coverage: `vitest --coverage`

## Notes
- Prioritize deterministic tests (pure helpers and query behavior) before external-network-dependent paths.
- Keep legal/compliance-sensitive behavior under explicit test protection (consent gating, de-identification, deletion paths).
