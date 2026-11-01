#[cfg(target_os = "macos")]
#[test]
fn native_discovery_keeps_visible_windows_and_filters_only_verified_offscreen_surfaces() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-macos-menu-region.py");
    assert!(std::process::Command::new("python3")
        .arg(script)
        .status()
        .expect("Run native discovery classification regression tests")
        .success());
}

#[cfg(target_os = "macos")]
#[test]
fn native_helper_refreshes_running_apps_and_exits_on_eof() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-macos-native.py");
    let status = std::process::Command::new("python3")
        .arg(script)
        .arg(env!("APP_FLASH_NATIVE"))
        .status()
        .expect("Run macOS native integration test");
    assert!(
        status.success(),
        "macOS native helper regression test failed"
    );
}
#[cfg(target_os = "macos")]
#[test]
fn idle_discovered_roots_are_not_starved_by_public_window_validation() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/test-macos-discovery-priority.py");
    assert!(std::process::Command::new("python3")
        .arg(script)
        .status()
        .expect("Run native candidate admission regression")
        .success());
}

#[cfg(target_os = "macos")]
#[test]
fn missing_system_focus_requires_complete_exact_local_window_evidence() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/test-macos-focus-confirmation.py");
    assert!(std::process::Command::new("python3")
        .arg(script)
        .status()
        .expect("Run exact local focus confirmation regression")
        .success());
}

#[cfg(target_os = "macos")]
#[test]
fn fullscreen_departure_never_hops_between_apps_and_accepts_inactive_visible_source() {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools/test-macos-departure.py");
    assert!(std::process::Command::new("python3")
        .arg(script)
        .status()
        .expect("Run actual production departure control flow regression")
        .success());
}

#[cfg(target_os = "macos")]
#[test]
fn fullscreen_departure_waits_only_for_the_destination_display() {
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tools/test-macos-departure-visibility.py");
    assert!(std::process::Command::new("python3")
        .arg(script)
        .status()
        .expect("Run exact WindowServer per-display departure regression")
        .success());
}
