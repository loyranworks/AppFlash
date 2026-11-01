#[path = "../src/overlay_notice.rs"]
mod overlay_notice;
#[path = "../src/settings_guard.rs"]
mod settings_guard;

use overlay_notice::Notices;
use settings_guard::{SettingsGuard, UNSAVED_MESSAGE};
use std::time::{Duration, Instant};

#[test]
fn clean_settings_allow_keys_and_show_no_blocking_notice() {
    let guard = SettingsGuard::default();
    let mut notices = Notices::default();
    assert!(!guard.blocks_key(true, &mut notices, Instant::now()));
    assert!(!notices.present(Instant::now(), false).visible);
}

#[test]
fn unsaved_settings_block_keys_and_explain_how_to_resume_even_without_hint_enabled() {
    let now = Instant::now();
    let guard = SettingsGuard::default();
    let mut notices = Notices::default();
    guard.set_dirty(true, &mut notices);
    assert!(guard.blocks_key(true, &mut notices, now));
    let frame = notices.present(now, false);
    assert!(frame.visible);
    assert_eq!(
        frame.notice.unwrap().message,
        "设置尚未保存，快捷切换已暂停。请回到设置保存后再试。"
    );
    assert!(!notices.present(now + Duration::from_secs(6), false).visible);
    assert!(
        guard.is_dirty(),
        "Hiding or expiring a tip does not discard the draft"
    );
}

#[test]
fn repeated_key_down_and_key_up_do_not_restart_the_tip_animation() {
    let now = Instant::now();
    let guard = SettingsGuard::default();
    let mut notices = Notices::default();
    guard.set_dirty(true, &mut notices);
    assert!(guard.blocks_key(false, &mut notices, now));
    assert!(!notices.present(now, false).visible);
    assert!(guard.blocks_key(true, &mut notices, now));
    let first = notices.present(now, false).notice.unwrap().id;
    assert!(guard.blocks_key(true, &mut notices, now + Duration::from_millis(100)));
    assert!(guard.blocks_key(false, &mut notices, now + Duration::from_millis(200)));
    assert_eq!(notices.present(now, false).notice.unwrap().id, first);
}

#[test]
fn saving_or_reverting_clears_the_unsaved_tip_and_immediately_allows_keys() {
    let now = Instant::now();
    let guard = SettingsGuard::default();
    let mut notices = Notices::default();
    guard.set_dirty(true, &mut notices);
    guard.blocks_key(true, &mut notices, now);
    guard.set_dirty(false, &mut notices);
    assert!(!guard.blocks_key(true, &mut notices, now));
    assert!(!notices.present(now, false).visible);
}

#[test]
fn clearing_unsaved_state_preserves_an_unrelated_window_error() {
    let now = Instant::now();
    let guard = SettingsGuard::default();
    let mut notices = Notices::default();
    guard.set_dirty(true, &mut notices);
    notices.show("目标应用暂时没有响应", now);
    guard.set_dirty(false, &mut notices);
    assert_eq!(
        notices.present(now, false).notice.unwrap().message,
        "目标应用暂时没有响应"
    );
    assert_ne!(
        notices.present(now, false).notice.unwrap().message,
        UNSAVED_MESSAGE
    );
}
