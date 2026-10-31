use crate::{
    config::AppEntry,
    switching::{Action, Window},
};
use serde::Serialize;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[derive(Clone, Serialize)]
pub struct RunningApp {
    pub name: String,
    pub path: String,
}

pub struct Snapshot {
    pub windows: Vec<Window>,
    pub active: Option<u64>,
}

pub fn execute(entry: &AppEntry, action: Action) -> Result<(), String> {
    match action {
        Action::Launch => launch(entry),
        Action::Activate(id) => activate(id),
        #[cfg(target_os = "macos")]
        Action::Minimize(ids) => collapse(&ids),
        #[cfg(not(target_os = "macos"))]
        Action::Minimize(ids) => {
            let mut errors = Vec::new();
            for id in ids {
                if let Err(error) = minimize(id) {
                    errors.push(error);
                }
            }
            if errors.is_empty() {
                Ok(())
            } else {
                Err(errors.join("；"))
            }
        }
    }
}
