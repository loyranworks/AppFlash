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
