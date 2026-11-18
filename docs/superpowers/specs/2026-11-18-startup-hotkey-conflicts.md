# Startup hotkey conflict isolation

User report: AppFlash 0.1.17 on Windows fails at startup on Alt+KeyF and leaves every shortcut unavailable. Four exported startup records reproduce the same failure. GitHub main verified at 61fea223f5c42d03f6463c13b45bee492d7d77aa; the registration transaction and dependency versions did not change in 0.1.16/0.1.17.

Mini is a separate, resolved version mismatch: a running development 0.1.5 rejected hint.scale in current configuration. Starting the existing signed /Applications/AppFlash.app 0.1.17 restored switching, confirmed by the user, with original config bytes unchanged.

Fix startup and explicit resume by attempting each configured shortcut independently, tracking only successfully owned shortcuts, and reporting failed keys without disabling working keys. If every configured key fails, remain paused; an empty configuration is not an error. Preserve desired configuration on disk. Saving edited settings retains the existing atomic registration/persistence rollback so a failed edit never destroys the working subset. Pause releases exactly the actually registered subset. Warnings explain the distinction and preserve the OS error; do not claim to identify another app as the owner. No automatic key reassignment or retry polling.

Compared approaches: clearing all bindings is the existing failure; silently changing Alt+F would alter user intent; retain available bindings and clearly report unavailable keys. No changes to window focus/cycling, fullscreen, input admission, signing, updater keys, or OS permissions. The actual owner of Alt+F on the user's Windows machine remains unverified.
