#[path = "../src/switching.rs"]
mod switching;
use switching::{Action, Cycle, Window};

fn windows(ids: &[u64]) -> Vec<Window> {
    ids.iter()
        .map(|id| Window {
            id: *id,
            minimized: false,
        })
        .collect()
}

#[test]
fn launch_when_not_open() {
    assert_eq!(Cycle::default().next("app", &[], None), Action::Launch);
}

#[test]
fn single_window_toggle_and_restore() {
    let mut cycle = Cycle::default();
    assert_eq!(
        cycle.next("app", &windows(&[10]), None),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next("app", &windows(&[10]), Some(10)),
        Action::Minimize(vec![10])
    );
    assert_eq!(
        cycle.next("app", &windows(&[10]), None),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next(
            "app",
            &[Window {
                id: 10,
                minimized: true
            }],
            Some(10)
        ),
        Action::Activate(10)
    );
}

#[test]
fn cycles_stably_despite_z_order_changes_then_minimizes_all() {
    let mut cycle = Cycle::default();
    assert_eq!(
        cycle.next("app", &windows(&[30, 20, 10]), None),
        Action::Activate(30)
    );
    assert_eq!(
        cycle.next("app", &windows(&[20, 30, 10]), Some(30)),
        Action::Activate(20)
    );
    assert_eq!(
        cycle.next("app", &windows(&[10, 20, 30]), Some(20)),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next("app", &windows(&[10, 20, 30]), Some(10)),
        Action::Minimize(vec![30, 20, 10])
    );
    assert_eq!(
        cycle.next("app", &windows(&[10, 20, 30]), None),
        Action::Activate(30)
    );
}

#[test]
fn changing_app_resets_cycle_and_closed_windows_disappear() {
    let mut cycle = Cycle::default();
    cycle.next("first", &windows(&[1, 2]), None);
    assert_eq!(
        cycle.next("first", &windows(&[2, 3]), None),
        Action::Activate(2)
    );
    cycle.next("second", &windows(&[4]), None);
    assert_eq!(
        cycle.next("first", &windows(&[3, 2]), None),
        Action::Activate(3)
    );
}

#[test]
fn external_focus_uses_current_window_not_stale_cursor() {
    let mut cycle = Cycle::default();
    cycle.next("app", &windows(&[1, 2, 3]), None);
    assert_eq!(
        cycle.next("app", &windows(&[1, 2, 3]), Some(2)),
        Action::Activate(3)
    );
}
