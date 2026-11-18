#[path = "../src/bindings.rs"]
mod bindings;
use bindings::{transition, Registry};
use std::{cell::RefCell, collections::BTreeSet};

struct Fake {
    active: RefCell<BTreeSet<String>>,
    conflict: String,
}
impl Registry for Fake {
    fn register(&self, key: &str) -> Result<(), String> {
        if key == self.conflict {
            return Err("conflict".into());
        }
        if self.active.borrow().contains(key) {
            return Err("already registered".into());
        }
        self.active.borrow_mut().insert(key.into());
        Ok(())
    }
    fn unregister(&self, key: &str) -> Result<(), String> {
        self.active.borrow_mut().remove(key);
        Ok(())
    }
}

#[test]
fn conflict_preserves_previous_keys() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::from(["old".into()])),
        conflict: "bad".into(),
    };
    assert!(transition(
        &["old".into()],
        &["added".into(), "bad".into()],
        &registry,
        || Ok(())
    )
    .is_err());
    assert_eq!(*registry.active.borrow(), BTreeSet::from(["old".into()]));
}

#[test]
fn failed_save_restores_old_registration() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::from(["old".into()])),
        conflict: String::new(),
    };
    assert!(
        transition(&["old".into()], &["new".into()], &registry, || Err(
            "disk".into()
        ))
        .is_err()
    );
    assert_eq!(*registry.active.borrow(), BTreeSet::from(["old".into()]));
}

#[test]
fn replace_and_pause_without_restart() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::from(["same".into(), "old".into()])),
        conflict: String::new(),
    };
    transition(
        &["same".into(), "old".into()],
        &["same".into(), "new".into()],
        &registry,
        || Ok(()),
    )
    .unwrap();
    assert_eq!(
        *registry.active.borrow(),
        BTreeSet::from(["same".into(), "new".into()])
    );
    transition(&["same".into(), "new".into()], &[], &registry, || Ok(())).unwrap();
    assert!(registry.active.borrow().is_empty());
}

#[test]
fn startup_conflict_does_not_disable_other_shortcuts() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::new()),
        conflict: "Alt+KeyF".into(),
    };
    let outcome = bindings::enable_available(
        &[],
        &["Alt+KeyG".into(), "Alt+KeyF".into(), "Alt+KeyH".into()],
        &registry,
    );
    assert_eq!(outcome.registered, vec!["Alt+KeyG", "Alt+KeyH"]);
    assert_eq!(
        *registry.active.borrow(),
        BTreeSet::from(["Alt+KeyG".into(), "Alt+KeyH".into()])
    );
    assert!(!outcome.paused());
    let warning = outcome.warning().unwrap();
    assert!(warning.contains("Alt+KeyF"));
    assert!(warning.contains("2"));
    assert!(!warning.contains("原绑定保持不变"));
}

#[test]
fn all_shortcuts_blocked_stays_paused_but_empty_configuration_does_not() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::new()),
        conflict: "Alt+KeyF".into(),
    };
    let outcome = bindings::enable_available(&[], &["Alt+KeyF".into()], &registry);
    assert!(outcome.paused());
    assert!(outcome.warning().is_some());
    let empty = bindings::enable_available(&[], &[], &registry);
    assert!(!empty.paused());
    assert!(empty.warning().is_none());
}

#[test]
fn repeated_enable_retains_owned_keys_and_retries_previously_unavailable_keys() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::from(["Alt+KeyG".into()])),
        conflict: String::new(),
    };
    let outcome = bindings::enable_available(
        &["Alt+KeyG".into()],
        &["Alt+KeyG".into(), "Alt+KeyF".into()],
        &registry,
    );
    assert_eq!(outcome.registered, vec!["Alt+KeyG", "Alt+KeyF"]);
    assert!(outcome.warning().is_none());
    assert!(!outcome.paused());
}

#[test]
fn settings_save_failure_preserves_working_subset_from_partial_startup() {
    let registry = Fake {
        active: RefCell::new(BTreeSet::new()),
        conflict: "Alt+KeyF".into(),
    };
    let outcome =
        bindings::enable_available(&[], &["Alt+KeyG".into(), "Alt+KeyF".into()], &registry);
    assert!(transition(
        &outcome.registered,
        &["Alt+KeyH".into(), "Alt+KeyF".into()],
        &registry,
        || Ok(())
    )
    .is_err());
    assert_eq!(
        *registry.active.borrow(),
        BTreeSet::from(["Alt+KeyG".into()])
    );
}

struct SystemConflict {
    owned: RefCell<BTreeSet<String>>,
    blocked: String,
}
impl Registry for SystemConflict {
    fn describe(&self, key: &str) -> String {
        format!("AppFlash → Target [{key}]")
    }
    fn register(&self, key: &str) -> Result<(), String> {
        if key == self.blocked {
            return Err(format!("HotKey already registered: {key}"));
        }
        self.owned.borrow_mut().insert(key.into());
        Ok(())
    }
    fn unregister(&self, key: &str) -> Result<(), String> {
        self.owned.borrow_mut().remove(key);
        Ok(())
    }
}

#[test]
fn every_modifier_combination_and_letter_retains_other_keys_and_reports_unknown_owner() {
    let mods = ["Control", "Alt", "Shift", "Super"];
    for mask in 1..16 {
        let prefix = mods
            .iter()
            .enumerate()
            .filter(|(i, _)| mask & (1 << i) != 0)
            .map(|(_, m)| *m)
            .collect::<Vec<_>>()
            .join("+");
        for letter in b'A'..=b'Z' {
            let blocked = format!("{prefix}+Key{}", letter as char);
            let available = format!("{prefix}+Key{}", if letter == b'Z' { 'A' } else { 'Z' });
            let registry = SystemConflict {
                owned: RefCell::new(BTreeSet::new()),
                blocked: blocked.clone(),
            };
            let result =
                bindings::enable_available(&[], &[blocked.clone(), available.clone()], &registry);
            assert_eq!(result.registered, vec![available]);
            let message = result.warning().unwrap();
            assert!(message.contains(&blocked));
            assert!(message.contains("占用程序未知"), "{message}");
            assert!(message.contains("AppFlash → Target"));
        }
    }
}

#[test]
fn multiple_system_conflicts_are_all_reported_instead_of_only_the_first() {
    struct Several;
    impl Registry for Several {
        fn register(&self, key: &str) -> Result<(), String> {
            if key == "Alt+KeyG" {
                Ok(())
            } else {
                Err(format!("HotKey already registered: {key}"))
            }
        }
        fn unregister(&self, _: &str) -> Result<(), String> {
            Ok(())
        }
        fn describe(&self, key: &str) -> String {
            format!("AppFlash → Target [{key}]")
        }
    }
    let outcome = bindings::enable_available(
        &[],
        &["Alt+KeyA".into(), "Alt+KeyF".into(), "Alt+KeyG".into()],
        &Several,
    );
    assert_eq!(outcome.registered, vec!["Alt+KeyG"]);
    let warning = outcome.warning().unwrap();
    assert!(warning.contains("Alt+KeyA"));
    assert!(warning.contains("Alt+KeyF"));
    assert_eq!(warning.matches("占用程序未知").count(), 2);
}
