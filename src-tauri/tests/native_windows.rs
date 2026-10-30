#![cfg(windows)]
#[path = "../src/config.rs"]
mod config;
#[path = "../src/platform/mod.rs"]
mod platform;
#[path = "../src/switching.rs"]
mod switching;
use std::collections::HashSet;

#[test]
fn running_apps_have_unique_existing_paths_and_snapshot_is_scoped() {
    let began = std::time::Instant::now();
    let apps = platform::running_apps().unwrap();
    let keys: HashSet<_> = apps.iter().map(|app| config::path_key(&app.path)).collect();
    assert_eq!(keys.len(), apps.len());
    for app in &apps {
        assert!(std::path::Path::new(&app.path).is_absolute());
    }
    let impossible = platform::snapshot("C:\\app-flash-nonexistent\\uninstalled.exe").unwrap();
    assert!(impossible.windows.is_empty());
    println!(
        "Native scan: {} apps, {:?}; no window activation performed",
        apps.len(),
        began.elapsed()
    );
}

#[test]
fn extracts_real_explorer_icon() {
    use base64::Engine;
    let root = std::env::var("SystemRoot").unwrap();
    let icon = platform::icon(&format!("{root}\\explorer.exe")).unwrap();
    assert!(icon.starts_with("data:image/png;base64,iVBOR"));
    assert!(icon.len() > 100);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(icon.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    assert_eq!((reader.info().width, reader.info().height), (96, 96));
}
