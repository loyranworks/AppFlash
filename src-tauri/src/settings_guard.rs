use crate::overlay_notice::Notices;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub const UNSAVED_MESSAGE: &str = "设置尚未保存，快捷切换已暂停。请回到设置保存后再试。";

#[derive(Default)]
pub struct SettingsGuard {
    dirty: AtomicBool,
}

impl SettingsGuard {
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::SeqCst)
    }

    pub fn set_dirty(&self, dirty: bool, notices: &mut Notices) {
        self.dirty.store(dirty, Ordering::SeqCst);
        if !dirty {
            notices.clear_message(UNSAVED_MESSAGE);
        }
    }

    pub fn blocks_key(&self, pressed: bool, notices: &mut Notices, now: Instant) -> bool {
        if !self.is_dirty() {
            return false;
        }
        if pressed
            && notices
                .present(now, false)
                .notice
                .as_ref()
                .map(|notice| notice.message.as_str())
                != Some(UNSAVED_MESSAGE)
        {
            notices.show(UNSAVED_MESSAGE, now);
        }
        true
    }
}
