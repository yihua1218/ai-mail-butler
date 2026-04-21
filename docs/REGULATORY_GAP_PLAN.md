# Regulatory Gap Plan (Taiwan + United States + New York City)

This document summarizes practical product and engineering controls needed to improve compliance readiness.

## Scope Note
- This is an engineering readiness checklist, not legal advice.
- Final legal interpretation should be reviewed by qualified counsel.

## Taiwan (Personal Data Protection Act focus)

### Gaps to close
1. Consent evidence and traceability are not fully auditable by policy version.
2. Data-subject rights workflows are partial (deletion exists, broader rights are incomplete).
3. Retention windows are not policy-driven for all personal-data tables.

### Required hardening
1. Add `consent_audit_log` with policy version, source, timestamp, and actor metadata.
2. Implement DSAR endpoints for access/export/correction/restriction/withdrawal.
3. Add retention scheduler and legal-hold exceptions.
4. Document data inventory and purpose limitation mapping per table.

## United States (state privacy expectations)

### Gaps to close
1. No explicit "Do Not Sell/Share" controls/disclosure endpoint.
2. Cross-border transfer disclosures are not explicit in API-level reporting.
3. Minor-data safeguards are not first-class controls.

### Required hardening
1. Add user preference flags and API for "Do Not Sell/Share".
2. Add public privacy endpoint describing transfer destinations and processors.
3. Add age-gating and guardian-consent hooks where applicable.
4. Add data-processing purpose registry for internal auditability.

## New York City / New York State

### Gaps to close
1. Need stronger codified baseline controls under NY SHIELD-aligned practices.
2. Need prohibited-use control for NYC AEDT-sensitive use cases (e.g., employment decisioning).

### Required hardening
1. Enforce encryption at rest and tighten role-based access auditing.
2. Add incident-response playbook + tabletop validation schedule.
3. Add explicit "employment decision support prohibited" policy guardrails unless AEDT process is implemented.
4. Add export/run logs for training data with approver identity and purpose.

## Priority Roadmap

### Phase A (Immediate)
1. Consent audit table and DSAR export endpoint.
2. Retention scheduler for transcripts/feedback/logs.
3. Training export access logs and approver metadata.

### Phase B (Next)
1. Do Not Sell/Share controls and user-facing disclosure endpoint.
2. Cross-border transfer disclosure and processor inventory output.
3. Incident-response runbook publication and simulation logs.

### Phase C (Future)
1. Minor-data controls and guardian-consent flow.
2. NYC AEDT readiness mode if employment-related use cases are introduced.

## Validation Checklist
1. Unit tests for consent gating and retention deletion paths.
2. API tests for DSAR operations and authorization boundaries.
3. Security regression tests for export redaction and access logging.
4. Periodic compliance evidence export (monthly or quarterly).
