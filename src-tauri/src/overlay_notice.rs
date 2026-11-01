use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Clone, Serialize)]
pub struct Notice {
    pub id: u64,
    pub message: String,
}
pub struct Presentation {
    pub visible: bool,
    pub notice: Option<Notice>,
}
#[derive(Default)]
pub struct Notices {
    next: u64,
    current: Option<(Instant, Notice)>,
}
impl Notices {
    pub fn show(&mut self, message: &str, now: Instant) {
        if message.trim().is_empty() {
            self.clear();
            return;
        }
        self.next = self.next.wrapping_add(1);
        self.current = Some((
            now,
            Notice {
                id: self.next,
                message: message.chars().take(2048).collect(),
            },
        ));
    }
    pub fn clear(&mut self) {
        self.current = None;
    }
    pub fn clear_message(&mut self, message: &str) {
        if self
            .current
            .as_ref()
            .is_some_and(|(_, notice)| notice.message == message)
        {
            self.clear();
        }
    }
    pub fn present(&self, now: Instant, hint_ready: bool) -> Presentation {
        let notice = self
            .current
            .as_ref()
            .filter(|(since, _)| now.saturating_duration_since(*since) < Duration::from_secs(6))
            .map(|(_, notice)| notice.clone());
        Presentation {
            visible: hint_ready || notice.is_some(),
            notice,
        }
    }
}
