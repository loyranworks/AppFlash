#[allow(dead_code)]
#[path = "../src/diagnostics.rs"]
mod diagnostics;

use diagnostics::Logger;
use std::{
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
struct TempDirectory(PathBuf);
impl TempDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "appflash-diagnostics-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn january(day: u64, seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(1_735_689_600 + (day - 1) * 86_400 + seconds)
}

// Catches off-by-one retention, elapsed-duration retention, and deleting unrelated paths.
#[test]
fn retention_uses_seven_utc_calendar_days_and_preserves_unknown_paths() {
    let temp = TempDirectory::new();
    let logger = Logger::new(temp.0.clone(), "test-version").unwrap();
    for name in [
        "appflash-2024-12-31.log",
        "appflash-2025-01-01.log",
        "appflash-2025-01-02.log",
        "appflash-2025-01-08.log",
        "appflash-2025-01-09.log",
        "other.log",
        "appflash-2025-13-01.log",
        "appflash-2025-01-01.log.bak",
    ] {
        fs::write(temp.0.join(name), "sentinel").unwrap();
    }
    fs::create_dir(temp.0.join("appflash-2024-12-30.log")).unwrap();
    logger.maintenance_at(january(8, 0)).unwrap();
    assert!(!temp.0.join("appflash-2025-01-01.log").exists());
    assert!(!temp.0.join("appflash-2024-12-31.log").exists());
    for name in [
        "appflash-2025-01-02.log",
        "appflash-2025-01-08.log",
        "appflash-2025-01-09.log",
        "other.log",
        "appflash-2025-13-01.log",
        "appflash-2025-01-01.log.bak",
        "appflash-2024-12-30.log",
    ] {
        assert!(temp.0.join(name).exists(), "removed {name}");
    }
    logger.maintenance_at(january(9, 0)).unwrap();
    assert!(!temp.0.join("appflash-2025-01-02.log").exists());
}

// Catches daily cap overflow, repeated cap markers, and unbounded oversized messages.
#[test]
fn daily_cap_includes_one_marker_and_resets_at_midnight() {
    let temp = TempDirectory::new();
    let logger = Logger::new(temp.0.clone(), "test-version").unwrap();
    let huge = "界".repeat(100_000);
    for _ in 0..300 {
        logger
            .record_at(january(8, 86_399), "switch", &huge)
            .unwrap();
    }
    let path = temp.0.join("appflash-2025-01-08.log");
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.len() <= 2 * 1024 * 1024);
    assert!(content.len() > 1024 * 1024);
    assert_eq!(content.matches("daily 2 MiB limit reached").count(), 1);
    assert!(content.lines().all(|line| line.len() < 17_000));
    let length = content.len();
    drop(logger);
    let restarted = Logger::new(temp.0.clone(), "test-version").unwrap();
    restarted
        .record_at(january(8, 86_399), "switch", "after restart")
        .unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), length as u64);
    restarted
        .record_at(january(9, 0), "switch", "new day")
        .unwrap();
    assert!(fs::read_to_string(temp.0.join("appflash-2025-01-09.log"))
        .unwrap()
        .contains("new day"));
}

// Catches unsynchronized partial writes, newline injection, and UTF-8 truncation panics.
#[test]
fn concurrent_records_are_complete_sanitized_unicode_lines() {
    let temp = TempDirectory::new();
    let logger = Arc::new(Logger::new(temp.0.clone(), "test-version").unwrap());
    let threads: Vec<_> = (0..8)
        .map(|worker| {
            let logger = logger.clone();
            std::thread::spawn(move || {
                for entry in 0..40 {
                    logger
                        .record_at(
                            january(8, 12_345),
                            "切换\n\r\t\0",
                            &format!("worker={worker};entry={entry};你好\r\n下一行\u{2028}末尾"),
                        )
                        .unwrap();
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    let content = fs::read_to_string(temp.0.join("appflash-2025-01-08.log")).unwrap();
    assert_eq!(content.lines().count(), 320);
    assert!(!content.contains(['\r', '\t', '\0', '\u{2028}']));
    for worker in 0..8 {
        for entry in 0..40 {
            assert_eq!(
                content
                    .lines()
                    .filter(
                        |line| line.contains(&format!("worker={worker};entry={entry};你好"))
                            && line.ends_with("末尾")
                    )
                    .count(),
                1
            );
        }
    }
    assert!(content
        .lines()
        .all(|line| line.starts_with("2025-01-08T03:25:45Z [切换")));
}

// Catches directory-order export, including expired/future/unrelated data, and UTF-8 loss.
#[test]
fn export_has_context_and_only_retained_days_in_chronological_order() {
    let temp = TempDirectory::new();
    let logger = Logger::new(temp.0.clone(), "0.1.test").unwrap();
    for (day, text) in [
        (8, "last 你好"),
        (2, "first café"),
        (5, "middle 世界"),
        (1, "expired"),
        (9, "future"),
    ] {
        logger.record_at(january(day, 0), "switch", text).unwrap();
    }
    fs::write(temp.0.join("other.log"), "private unrelated content").unwrap();
    let output = temp.0.join("diagnostics.txt");
    logger.export_at(&output, january(8, 123)).unwrap();
    let content = fs::read_to_string(output).unwrap();
    assert!(
        content.contains("AppFlash") && content.contains("0.1.test") && content.contains("UTC")
    );
    assert!(content.contains(std::env::consts::OS) && content.contains(std::env::consts::ARCH));
    assert!(content.find("first café").unwrap() < content.find("middle 世界").unwrap());
    assert!(content.find("middle 世界").unwrap() < content.find("last 你好").unwrap());
    assert!(
        !content.contains("expired")
            && !content.contains("future")
            && !content.contains("private unrelated")
    );
}

// Catches swallowed IO failures and diagnostics writing through an invalid directory.
#[test]
fn invalid_paths_report_errors_without_panicking() {
    let temp = TempDirectory::new();
    let logger = Logger::new(temp.0.clone(), "test-version").unwrap();
    logger
        .record_at(january(8, 0), "switch", "success")
        .unwrap();
    assert!(logger.export_at(&temp.0, january(8, 0)).is_err());
    assert!(logger
        .export_at(&temp.0.join("missing/report.txt"), january(8, 0))
        .is_err());
    let invalid_directory = temp.0.join("plain-file");
    fs::write(&invalid_directory, "keep").unwrap();
    assert!(Logger::new(invalid_directory.clone(), "test-version").is_err());
    fs::remove_file(invalid_directory).unwrap();
    fs::remove_file(temp.0.join("appflash-2025-01-08.log")).unwrap();
    fs::remove_dir(&temp.0).unwrap();
    fs::write(&temp.0, "directory replaced by a file").unwrap();
    assert!(logger
        .record_at(january(8, 0), "switch", "failure")
        .is_err());
    assert!(logger.maintenance_at(january(8, 0)).is_err());
    assert!(logger
        .export_at(&temp.0.join("report.txt"), january(8, 0))
        .is_err());
    fs::remove_file(&temp.0).unwrap();
}

// Catches cleanup following a same-name symlink and writes modifying its external target.
#[cfg(unix)]
#[test]
fn owned_looking_symlinks_are_preserved_and_never_followed() {
    use std::os::unix::fs::symlink;
    let temp = TempDirectory::new();
    let target = temp.0.join("unrelated.txt");
    fs::write(&target, "untouched").unwrap();
    let logger = Logger::new(temp.0.clone(), "test-version").unwrap();
    symlink(&target, temp.0.join("appflash-2025-01-01.log")).unwrap();
    symlink(&target, temp.0.join("appflash-2025-01-08.log")).unwrap();
    logger.maintenance_at(january(8, 0)).unwrap();
    assert!(temp
        .0
        .join("appflash-2025-01-01.log")
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(logger
        .record_at(january(8, 0), "switch", "overwrite")
        .is_err());
    logger
        .export_at(&temp.0.join("report.txt"), january(8, 0))
        .unwrap();
    assert!(!fs::read_to_string(temp.0.join("report.txt"))
        .unwrap()
        .contains("untouched"));
    assert_eq!(fs::read_to_string(target).unwrap(), "untouched");
}
