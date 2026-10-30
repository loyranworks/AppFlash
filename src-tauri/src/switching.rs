#[derive(Clone, Debug)]
pub struct Window {
    pub id: u64,
    pub minimized: bool,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    Launch,
    Activate(u64),
    Minimize(Vec<u64>),
}

#[derive(Default)]
pub struct Cycle {
    app: String,
    order: Vec<u64>,
    last: Option<u64>,
}

impl Cycle {
    pub fn next(&mut self, app: &str, windows: &[Window], active: Option<u64>) -> Action {
        if self.app != app {
            self.app = app.to_owned();
            self.order.clear();
            self.last = None;
        }
        self.order
            .retain(|id| windows.iter().any(|window| window.id == *id));
        for window in windows {
            if !self.order.contains(&window.id) {
                self.order.push(window.id);
            }
        }
        if windows.is_empty() {
            self.last = None;
            return Action::Launch;
        }
        let foreground = windows
            .iter()
            .find(|window| Some(window.id) == active && !window.minimized);
        if let Some(window) = foreground {
            let position = self.order.iter().position(|id| *id == window.id).unwrap();
            if position + 1 == self.order.len() {
                self.last = None;
                return Action::Minimize(self.order.clone());
            }
            self.last = Some(self.order[position + 1]);
        } else {
            self.last = self
                .last
                .filter(|id| self.order.contains(id))
                .or(Some(self.order[0]));
        }
        Action::Activate(self.last.unwrap())
    }
}
