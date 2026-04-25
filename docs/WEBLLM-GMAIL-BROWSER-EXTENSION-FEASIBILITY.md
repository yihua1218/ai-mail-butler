# WebLLM Gmail Browser Extension Feasibility Assessment

## Executive Summary

This document evaluates a product design where a Chrome/Edge browser extension runs a local large language model through WebLLM and helps the user read, draft, and optionally send Gmail replies without routing message content to a third-party inference server.

The concept is technically viable, but only under a constrained rollout model. A local-first draft assistant is highly feasible. Fully autonomous reply sending is moderately feasible only after the extension adds strong safeguards around authorization, browser lifecycle, model performance, and failure handling.

## Target Outcome

- Let the user keep email content, prompt context, rules, and tone profiles inside the browser on the user's device.
- Use WebLLM for local inference so message content is not sent to an external AI provider.
- Provide a Gmail-focused browser extension that can help read inbox content, draft replies, and optionally send replies on the user's behalf.
- Allow the user to define rules, tone, approval thresholds, and automation boundaries locally.

## Scope and Assumptions

### In Scope

- Chrome and Microsoft Edge only for v1.
- Gmail web experience as the primary user surface.
- WebLLM running in-browser with WebGPU acceleration.
- All configuration, rule definitions, prompt templates, and local audit data stored in browser-local storage.
- User-authorized browser operation for inbox inspection, draft generation, and sending.

### Out of Scope

- Firefox, Safari, and mobile browsers.
- Server-side inference.
- Multi-tenant cloud sync of prompts, settings, or message history.
- Full enterprise admin integration in v1.

### Operating Assumptions

1. The user is already signed into Gmail in the browser profile where the extension is installed.
2. The user accepts a local extension permission model for reading page content and storing settings locally.
3. The browser and device support WebGPU well enough for at least one supported WebLLM model.
4. V1 defaults to draft-first behavior, with auto-send enabled only for user-approved rule subsets.

## Desired End State

### Product Goals

1. Read relevant Gmail conversation context with minimal user friction.
2. Produce replies in the user's preferred tone and structure using local inference.
3. Keep settings and email-derived working data local to the browser profile.
4. Preserve enough reliability that the extension can be used daily without constant manual recovery.

### Privacy Goals

1. No email body content is sent to a remote model inference API.
2. No centralized application server is required for message analysis or reply generation.
3. Logging stays local by default, with redaction and retention controls.
4. The user can inspect and delete local data at any time.

## Feasibility Summary

| Dimension                            | Rating      | Notes                                                                                                                            |
| :----------------------------------- | :---------- | :------------------------------------------------------------------------------------------------------------------------------- |
| Local inference with WebLLM          | High        | WebLLM explicitly supports Chrome extensions, dedicated workers, and service-worker usage.                                       |
| Local-only settings and rule storage | High        | `chrome.storage.local`, IndexedDB, and Cache API are sufficient for settings, prompts, and model caches.                         |
| Gmail draft assistance               | High        | A content-script-assisted workflow can read visible thread content and prepare draft replies reliably enough for supervised use. |
| Gmail API integration                | Medium      | Technically strong, but requires explicit token flows and a more formal authorization path.                                      |
| Fully automated auto-send            | Medium-Low  | Feasible only with robust safety gates, lifecycle recovery, and careful trust boundaries.                                        |
| Privacy promise                      | Medium-High | Strong for AI processing, but still bounded by Gmail itself, extension permissions, and browser storage security.                |
| Chrome Web Store readiness           | Medium      | Possible, but extension behavior must be narrowly scoped and transparent to survive review.                                      |

## Candidate Architectures

### Option A: Gmail API plus `chrome.identity` plus WebLLM

#### Option A Flow

1. The extension obtains a Gmail access token through the browser identity flow.
2. It lists threads, fetches message bodies, and creates drafts or sends replies through Gmail API endpoints.
3. Message content is summarized and replied to locally through WebLLM.
4. Local rules determine whether a reply becomes a draft or is sent automatically.

#### Option A Strengths

- Most durable and structured integration path.
- Clear access to threads, labels, drafts, and message metadata.
- Easier to reason about idempotency, retry logic, and thread association.
- Better long-term maintainability than DOM automation.

#### Option A Weaknesses

- Requires a full auth flow and visible permissions UX.
- More product friction during initial onboarding.
- May increase store-review and consent scrutiny because mailbox access is explicit.

#### Option A Assessment

This is the best engineering architecture if the product is intended to become stable and maintainable. It should be the target architecture for production-quality behavior.

### Option B: Gmail Web UI Automation plus Content Script plus WebLLM

#### Option B Flow

1. A content script runs on `mail.google.com`.
2. It reads thread content from the currently open conversation or inbox listing.
3. A side panel or floating compose assistant asks WebLLM to produce a reply locally.
4. The extension fills the Gmail compose box or triggers the native send action when policy permits.

#### Option B Strengths

- No separate server-side integration required.
- Faster to prototype.
- Works as a user-facing browser assistant even if API-based onboarding is deferred.

#### Option B Weaknesses

- DOM structure changes can break the extension.
- Harder to support background inbox scanning reliably.
- Harder to create deterministic automation around thread state.
- Greater risk of silent failure when Gmail UI changes.

#### Option B Assessment

This is the fastest path to an MVP and a good path for supervised draft assistance, but it is not the strongest foundation for unattended automation.

### Option C: Hybrid Design

#### Option C Flow

1. Use API access for durable mailbox operations when available.
2. Use content scripts for in-page compose assistance, inline controls, and user-visible review UX.
3. Keep WebLLM and rule evaluation local in the extension runtime.

#### Option C Strengths

- Best combination of user experience and reliability.
- In-page assistant can remain intuitive while mailbox operations become structured.
- Allows gradual rollout from supervised drafting to selective automation.

#### Option C Weaknesses

- Highest engineering complexity.
- Requires careful reconciliation between API state and in-page UI state.

#### Option C Assessment

This is the recommended target architecture. It allows a realistic product path: start with UI assistance, then add API-backed durability where needed.

## Recommended Direction

Adopt Option C as the strategic architecture, but sequence delivery as follows:

1. Phase 0: prove WebLLM performance and memory behavior in a Chrome/Edge extension.
2. Phase 1: ship supervised Gmail web UI drafting with no unattended send.
3. Phase 2: add API-backed draft creation and deterministic thread handling.
4. Phase 3: enable selective auto-send only for high-confidence, tightly scoped rule classes.

## Detailed V1 Product Specification

### Primary User Experience

1. The user installs the extension and opens Gmail in Chrome or Edge.
2. The extension opens a side panel for model setup, rules, tone presets, and safety settings.
3. The user selects a local model and waits for the first download and warm-up.
4. When the user opens a thread, the extension offers these actions: summarize thread, suggest reply, suggest short reply, create draft from rule, and mark the sender as automated-safe or human-review-only.
5. The generated reply is shown side-by-side with explanation, confidence, matched rule, and send mode.
6. By default, the user approves the reply before it is inserted or sent.

### Optional Automation Modes

- `manual`: only suggest text; user copies or inserts it manually.
- `draft_assist`: extension writes a Gmail draft but never sends automatically.
- `guarded_autosend`: extension may send automatically only for explicitly allowlisted rules and confidence thresholds.
- `disabled_for_sender`: never automate for this sender, label, or domain.

### Rule Engine Requirements

Each rule should support the following fields:

| Field                      | Purpose                              |
| :------------------------- | :----------------------------------- |
| `id`                       | Stable local identifier              |
| `enabled`                  | Toggle rule on or off                |
| `priority`                 | Resolve rule conflicts               |
| `match.sender`             | Email or domain matching             |
| `match.labels`             | Gmail labels or inferred labels      |
| `match.subject_regex`      | Thread title pattern                 |
| `match.contains`           | Keyword or phrase matching           |
| `tone_profile_id`          | Reference to writing style           |
| `instruction_template`     | User-authored guidance for the model |
| `allowed_actions`          | Draft only or auto-send              |
| `min_confidence_auto_send` | Minimum confidence threshold         |
| `cooldown_minutes`         | Prevent repeated actions             |
| `requires_human_review`    | Hard override for sensitive threads  |

### Tone Profile Requirements

Each tone profile should support:

- Display name
- System prompt template
- Preferred greeting and sign-off
- Allowed language list
- Preferred message length
- Formality level
- Whether to ask clarifying questions or avoid them
- Whether to avoid commitments without user approval

### Safety Controls

1. Default every new rule to `draft_assist`.
2. Block auto-send for messages containing attachments, invoices, legal keywords, HR topics, refund disputes, or medical terms until explicitly allowlisted.
3. Require a local cooldown to prevent repeated auto-replies in the same thread.
4. Show an explanation panel that lists matched rules, sender, confidence, and why the model chose the reply.
5. Log local decision summaries without storing full plaintext thread content unless the user opts in.

## Technical Architecture

### Recommended Runtime Layout

- `extension service worker`: orchestrates alarms, rule scheduling, token refresh state, and background events.
- `content script`: reads Gmail UI state, injects controls, and bridges compose actions.
- `side panel`: settings UI, model selector, audit viewer, and recovery actions.
- `WebLLM worker or service worker`: runs model loading and inference off the UI thread.
- `IndexedDB` plus `chrome.storage.local`: persist rules, settings, tone profiles, and local audit metadata.
- `Cache API`: store model artifacts when using WebLLM cache backends.

### Why This Split Matters

- Manifest V3 service workers are event-driven and can be unloaded when idle.
- Heavy inference should not run in the content script.
- Model loading must survive UI transitions and should degrade safely if the browser unloads the worker.

### Message Processing Pipeline

1. A trigger fires: thread opened, compose opened, alarm tick, or user command.
2. Content is normalized into a local prompt bundle.
3. The rule engine decides mode, tone profile, and whether the thread is eligible.
4. WebLLM generates a structured result containing reply text, confidence score, sensitivity flags, and a recommended action.
5. The policy layer decides one of: ignore, show suggestion, create draft, or send.
6. The local audit layer records metadata and outcome.

## Local Data Model

### Local Storage Categories

| Category                 | Store                  | Notes                                     |
| :----------------------- | :--------------------- | :---------------------------------------- |
| Extension settings       | `chrome.storage.local` | Small config, toggles, onboarding state   |
| Rules and tone profiles  | IndexedDB              | Better schema evolution and query support |
| Model artifacts          | Cache API or IndexedDB | Managed by WebLLM cache backend           |
| Local audit metadata     | IndexedDB              | Redacted by default                       |
| Temporary prompt bundles | Memory only            | Cleared after request completion          |

### Strong Recommendation

Encrypt sensitive local state at rest using WebCrypto and a user-defined local passphrase for:

- Rule templates containing sensitive business logic
- Allowlists and sender classifications
- Audit metadata beyond minimal telemetry

## Privacy Boundary Assessment

### What Stays Local

- Prompt assembly
- Model inference
- User-defined rules and tone profiles
- Confidence scoring and automation decisions
- Local audit records if enabled

### What Does Not Change

- Gmail message content still exists within Google's own product and infrastructure because Gmail is the mail provider.
- If model weights are downloaded from a remote host, the browser still contacts that host for artifacts, though not for message content.
- Browser extensions with mailbox-reading permissions remain a high-trust component on the local machine.

### Privacy Claim That Is Defensible

The product can credibly claim that reply generation and automation logic run locally in the browser and that email content is not forwarded to a third-party AI inference service, provided telemetry is off by default and debug logs are redacted.

## Main Risks and Required Mitigations

| Risk                                          | Impact | Likelihood | Mitigation                                                                                                       |
| :-------------------------------------------- | :----- | :--------- | :--------------------------------------------------------------------------------------------------------------- |
| WebGPU unavailable or unstable on user device | High   | Medium     | Offer a compatibility check and fall back to supervised mode when no supported runtime exists.                   |
| Model cold start is too slow                  | High   | High       | Start with smaller WebLLM-supported models, prewarm on demand, cache aggressively, and show progress explicitly. |
| Extension worker unload interrupts inference  | Medium | High       | Keep inference in a dedicated worker or service-worker flow with retry-safe state restoration.                   |
| Gmail UI changes break DOM automation         | High   | Medium     | Keep UI automation limited, add selector health checks, and shift durable operations to the API path.            |
| Incorrect auto-reply in a sensitive thread    | High   | Medium     | Default to draft-first, add content risk filters, and require allowlisted rules for auto-send.                   |
| Local storage leakage on a shared machine     | Medium | Medium     | Support passphrase-based encryption and one-click local wipe.                                                    |
| Store review or permission rejection          | High   | Medium     | Narrow permissions, explain user benefit clearly, and provide transparent in-product disclosure.                 |
| Model quality is too low for nuanced replies  | Medium | High       | Constrain initial use cases to repetitive email classes and expose human review mode.                            |

## Engineering Challenges To Overcome

### 1. Browser Runtime Constraints

Manifest V3 background execution is not persistent. The extension must tolerate service-worker suspension, tab closure, and model reload costs.

### 2. Model Size and Latency

WebLLM can run locally, but the experience depends on model size, quantization, GPU quality, available memory, and first-load artifact download time.

### 3. Structured Output Reliability

For automation, free-form text is not enough. The extension should ask the model for structured JSON containing reply text, confidence, sensitivity flags, and recommended action.

### 4. Safe Send Boundaries

Sending an email is a high-impact action. The policy layer must remain separate from the model and should never let the model decide send authority by itself.

### 5. Gmail Access Strategy

Pure DOM automation is fragile. Pure API access improves reliability but adds onboarding and permission complexity. The product must choose a staged hybrid path.

## Practical Feasibility by Milestone

### Phase 0: Technical Spike

Objective: prove runtime feasibility.

Deliverables:

1. Load one WebLLM model inside a Chrome or Edge extension.
2. Demonstrate thread summarization on a visible Gmail thread.
3. Measure cold start, warm response latency, memory use, and worker recovery.

Exit Criteria:

1. The model initializes successfully on at least one common Chrome and one Edge test setup.
2. Median warm reply generation is acceptable for assisted drafting.
3. The extension survives page refresh without corrupting local state.

### Phase 1: Supervised Draft Assistant

Objective: ship a safe local-first MVP.

Deliverables:

1. A side panel UI for rules, tone profiles, and model status.
2. Content-script thread extraction and draft insertion.
3. Local audit metadata and one-click delete controls.

Exit Criteria:

1. The user can generate draft replies for selected threads.
2. No auto-send path exists yet.
3. All settings remain local to the browser profile.

### Phase 2: Durable Draft Workflow

Objective: improve reliability for real usage.

Deliverables:

1. API-backed draft creation where available.
2. Deterministic thread matching and cooldown enforcement.
3. Structured JSON generation with safety classification.

Exit Criteria:

1. Draft creation is reproducible and resilient across Gmail UI changes.
2. The rule engine correctly blocks sensitive threads.
3. Recovery after worker suspension is deterministic.

### Phase 3: Selective Guarded Auto-Send

Objective: permit narrow automation with strong controls.

Deliverables:

1. Allowlisted sender and rule classes.
2. Confidence thresholding and a safety veto layer.
3. A local audit trail for auto-send decisions.

Exit Criteria:

1. Auto-send is enabled only for explicitly approved rule classes.
2. Sensitive-content detection blocks unsafe threads.
3. The user can disable automation instantly and wipe local state.

## Final Assessment

### Overall Feasibility

The concept is feasible.

### Feasibility by Product Shape

- Local Gmail drafting assistant: High
- Local rule-driven reply generation: High
- Background mailbox triage in the browser: Medium
- Trustworthy full auto-reply without review: Medium-Low

### Why the Feasibility Is Not Higher

The main constraints are not whether WebLLM can run locally. It can. The harder problems are extension lifecycle control, user-device performance variance, Gmail integration durability, and designing a safety layer that is strong enough for unattended sending.

## Recommendation

Build this product, but do not start with autonomous sending.

Recommended launch order:

1. Start with a local-only supervised draft assistant.
2. Narrow the first use cases to repetitive, low-risk message classes.
3. Add durable integration and structured safety gating before enabling auto-send.
4. Treat fully automatic sending as an advanced mode with explicit local opt-in and reversible controls.

## Validation Checklist

1. Verify WebLLM model startup, recovery, and warm latency inside a Manifest V3 extension.
2. Verify all settings and rule data stay local and can be deleted on demand.
3. Verify no debug or telemetry path transmits message body content externally by default.
4. Verify sensitive-thread detection prevents auto-send for blocked categories.
5. Verify Gmail UI changes fail closed instead of sending malformed or partial replies.
