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
