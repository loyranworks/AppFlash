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
- [x] Commit and provide a reviewable patch. Do not present a source fix as installed on remote Windows machines.

## Verification boundaries

Mini recovery is user-confirmed. The Windows startup failure is provided by user logs; local tests model the unavailable OS registration and exercise production reconciliation. Actual Alt+F ownership on Windows is not known. A release is separate from these source changes.

## Results

- Isolated regression starts from the exact reported Alt+F error and retains successful G/H registrations. Existing save-conflict and persistence rollback tests remain green.
- UI warning/clearing and all-blocked pause behavior covered; production callback extraction tests compile both platform selections and reject unavailable keys before admission.
- First full run exposed an incomplete input callback fixture (registered field missing); fixture corrected and expanded, then all 92 Rust tests passed. All 62 JavaScript tests passed, including fixed certificate/signature tests; formatter and release configuration checks passed.
- Candidate version 0.1.18; installed Mini remains signed 0.1.17 and is user-confirmed working. No release tag or remote installation is included.

- Final candidate source check: cargo check --locked and formatting passed; final full run again passed all 62 JavaScript and 92 Rust tests after the version update.
- Reviewable draft: https://github.com/loyranworks/AppFlash/pull/1 (base main 61fea223; code commit 44642c3). Main and client release remain 0.1.17; no v0.1.18 tag was pushed.

## Conflict-detail follow-up

- User clarified that there are no duplicate assignments in AppFlash. No internal-duplicate flow, configuration reset/deletion, or automatic rebind was added. One Windows AppFlash process was reported.
- After exiting AppFlash, the user's separate RegisterHotKey probe still returned Win32 1409. This demonstrates a system-level registration outside the exited application; the owning program is still unknown.
- Diagnostics now label the configured AppFlash target and display combination on registration, release, and rollback errors. An explicit AlreadyRegistered result reports an external owner as unknown; other errors remain generic registration failures.
- Added exhaustive pure registration coverage for 15 nonempty modifier combinations × 26 letters (390 cases), multiple simultaneous conflicts, actual configuration label lookup without mutation, and English diagnostics.
- Latest full run: 63 JavaScript and 95 Rust tests passed, including fixed-signature/update checks.

Resolution: user identified a separate self-authored shortcut utility launching at login and competing for Alt+F. Configuration files were not reset/deleted. The protection and contextual diagnostics remain a draft patch; no new release or remote installation was requested/performed.
