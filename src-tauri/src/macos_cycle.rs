use crate::switching::{Action, Window};

/// Clone before planning and commit only after the native action succeeds.
/// macOS focus snapshots may lag behind successful window operations, so round
/// progress is based on visits rather than the last observed foreground window.
#[derive(Clone, Default)]
pub struct Cycle {
    app: String,
    order: Vec<u64>,
    visited: Vec<u64>,
    observed_active: Option<u64>,
    ignore_active_once: bool,
}

impl Cycle {
    pub fn next(&mut self, app: &str, windows: &[Window], active: Option<u64>) -> Action {
        if self.app != app {
            *self = Self {
                app: app.to_owned(),
                ..Self::default()
            };
        }
        self.order
            .retain(|id| windows.iter().any(|window| window.id == *id));
        self.visited.retain(|id| self.order.contains(id));
        for window in windows {
            if !self.order.contains(&window.id) {
                self.order.push(window.id);
            }
        }
        if self.order.is_empty() {
            self.ignore_active_once = false;
            self.observed_active = None;
            return Action::Launch;
        }

        // A successful collapse can still be followed by the previous active
        // window in AX. The next round must restore its first window anyway.
        let focus_changed = self.observed_active != active;
        self.observed_active = active;
        if !self.ignore_active_once && focus_changed {
            if let Some(window) = windows
                .iter()
                .find(|window| Some(window.id) == active && !window.minimized)
            {
                if !self.visited.contains(&window.id) {
                    self.visited.push(window.id);
                }
            }
        }
        self.ignore_active_once = false;

        if let Some(id) = self.order.iter().find(|id| !self.visited.contains(id)) {
            self.visited.push(*id);
            Action::Activate(*id)
        } else {
            self.visited.clear();
            self.ignore_active_once = true;
            Action::Minimize(self.order.clone())
        }
    }
}
