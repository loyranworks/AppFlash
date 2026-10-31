#[path = "../src/overlay_notice.rs"]
mod overlay_notice;
use overlay_notice::Notices;
use std::time::{Duration, Instant};

#[test]
fn errors_remain_visible_without_the_prefix_for_six_seconds() {
    let now = Instant::now();
    let mut notices = Notices::default();
    notices.show("目标应用暂时没有响应", now);
    let frame = notices.present(now, false);
    assert!(frame.visible);
    assert_eq!(frame.notice.unwrap().message, "目标应用暂时没有响应");
    assert!(
        notices
            .present(now + Duration::from_millis(5999), false)
            .visible
    );
    assert!(!notices.present(now + Duration::from_secs(6), false).visible);
}

#[test]
fn latest_error_replaces_the_previous_one_and_restarts_its_lifetime() {
    let now = Instant::now();
    let mut notices = Notices::default();
    notices.show("first", now);
    let first = notices.present(now, false).notice.unwrap();
    notices.show("second", now + Duration::from_secs(5));
    let latest = notices
        .present(now + Duration::from_secs(6), false)
        .notice
        .unwrap();
    assert_eq!(latest.message, "second");
    assert_ne!(latest.id, first.id);
    assert!(
        !notices
            .present(now + Duration::from_secs(11), false)
            .visible
    );
}

#[test]
fn success_clears_the_error_without_hiding_a_requested_hint() {
    let now = Instant::now();
    let mut notices = Notices::default();
    notices.show("error", now);
    notices.clear();
    assert!(!notices.present(now, false).visible);
    let hint = notices.present(now, true);
    assert!(hint.visible);
    assert!(hint.notice.is_none());
}
