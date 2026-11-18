# Startup hotkey conflict fix

> Execute inline; no delegation requested by the user.

**Goal:** One unavailable global shortcut must not disable other configured shortcuts at startup or resume.
**Spec:** docs/superpowers/specs/2026-11-18-startup-hotkey-conflicts.md
**Architecture:** Keep atomic settings transactions; add a pure best-effort enable result for startup/resume, and update runtime/UI warning state from actual registrations.

- [x] Verify latest GitHub source and identify separate Mini version mismatch without changing user configuration.
- [x] Reproduce all-or-nothing startup failure and write regression cases before implementation.
- [x] Integrate enable_available into initial startup/resume; pause and save use tracked successful keys, unreadable config remains protected.
- [x] Preserve error visibility/clearing in UI and add English translations and regression coverage.
- [x] Run all JavaScript and Rust tests, format/type/build checks appropriate to the patch; review the exact diff.
- [ ] Commit and provide a reviewable patch. Do not present a source fix as installed on remote Windows machines.

## Verification boundaries

Mini recovery is user-confirmed. The Windows startup failure is provided by user logs; local tests model the unavailable OS registration and exercise production reconciliation. Actual Alt+F ownership on Windows is not known. A release is separate from these source changes.

## Results

- Isolated regression starts from the exact reported Alt+F error and retains successful G/H registrations. Existing save-conflict and persistence rollback tests remain green.
- UI warning/clearing and all-blocked pause behavior covered; production callback extraction tests compile both platform selections and reject unavailable keys before admission.
- First full run exposed an incomplete input callback fixture (registered field missing); fixture corrected and expanded, then all 92 Rust tests passed. All 62 JavaScript tests passed, including fixed certificate/signature tests; formatter and release configuration checks passed.
- Candidate version 0.1.18; installed Mini remains signed 0.1.17 and is user-confirmed working. No release tag or remote installation is included.
