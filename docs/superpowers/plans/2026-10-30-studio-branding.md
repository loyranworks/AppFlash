# Loyran Works branding implementation plan

**Goal:** Apply the approved bilingual studio signature below APP / FLASH and in About AppFlash.

**Design:** Keep AppFlash identity and application slogan unchanged. Display only the selected interface language: 落雁工作室 / 巧思成形，琢而至美。 or loyran works. / Make Ideas Work Beautifully. Use VI-supplied Noto Serif SC and Playfair Display for the studio name, restrained navy text and compact spacing. Use the existing native About menu with localized studio text in Windows comments and macOS credits.

**Scope:** ui/index.html, settings.css, i18n.js, branding tests, font assets and OFL licenses, src-tauri/src/main.rs, README files and verification notes. No configuration migration, icon replacement, global font change, remote release or installed-process replacement.

- [x] Write and run failing branding/localization/menu tests.
- [x] Subset the supplied fonts to the fixed studio text, preserve OFL licenses, add sidebar markup and localization.
- [x] Add localized native About metadata and menu entry using the current package version.
- [x] Test translations, assets, native metadata, compile and rebuild signed Windows installer; record macOS/visual checks still pending.

Verification: 22 JavaScript and 19 Rust tests passed, cargo check and formatting/diff checks passed. NSIS 0.1.3 rebuilt successfully and verified with the existing updater public key; modified bytes rejected. Browser UI host unavailable; no real tray click, macOS execution or installed-client replacement performed.
