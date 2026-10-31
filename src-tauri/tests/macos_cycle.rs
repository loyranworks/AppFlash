#[path = "../src/macos_cycle.rs"]
mod macos_cycle;
#[allow(dead_code)]
#[path = "../src/switching.rs"]
mod switching;
use macos_cycle::Cycle;
use switching::{Action, Window};

fn windows(ids: &[u64]) -> Vec<Window> {
    ids.iter()
        .map(|id| Window {
            id: *id,
            minimized: false,
        })
        .collect()
}

#[test]
fn initially_active_last_window_does_not_end_the_round_early() {
    let mut cycle = Cycle::default();
    let snapshot = windows(&[10, 20, 30]);
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(10));
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(20));
    assert_eq!(
        cycle.next("app", &snapshot, Some(20)),
        Action::Minimize(vec![10, 20, 30])
    );
}

#[test]
fn repeated_stale_focus_advances_after_each_successful_activation() {
    let mut cycle = Cycle::default();
    let snapshot = windows(&[10, 20, 30]);
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(20));
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(30));
    assert_eq!(
        cycle.next("app", &snapshot, Some(10)),
        Action::Minimize(vec![10, 20, 30])
    );
}

#[test]
fn no_observed_focus_still_completes_a_round() {
    let mut cycle = Cycle::default();
    let snapshot = windows(&[10, 20]);
    assert_eq!(cycle.next("app", &snapshot, None), Action::Activate(10));
    assert_eq!(cycle.next("app", &snapshot, None), Action::Activate(20));
    assert_eq!(
        cycle.next("app", &snapshot, None),
        Action::Minimize(vec![10, 20])
    );
}

#[test]
fn new_closed_and_reordered_windows_preserve_remaining_round_order() {
    let mut cycle = Cycle::default();
    assert_eq!(
        cycle.next("app", &windows(&[10, 20, 30]), None),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next("app", &windows(&[30, 40, 10, 20]), Some(10)),
        Action::Activate(20)
    );
    // Closing 30 removes it; the new 40 still needs one successful activation.
    assert_eq!(
        cycle.next("app", &windows(&[40, 20, 10]), Some(20)),
        Action::Activate(40)
    );
    assert_eq!(
        cycle.next("app", &windows(&[20, 40, 10]), Some(40)),
        Action::Minimize(vec![10, 20, 40])
    );
}

#[test]
fn manual_focus_of_unvisited_window_counts_as_a_visit() {
    let mut cycle = Cycle::default();
    assert_eq!(
        cycle.next("app", &windows(&[10, 20, 30]), None),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next("app", &windows(&[30, 10, 20]), Some(30)),
        Action::Activate(20)
    );
    assert_eq!(
        cycle.next("app", &windows(&[20, 30, 10]), Some(20)),
        Action::Minimize(vec![10, 20, 30])
    );
}

#[test]
fn collapsed_round_restores_first_window_even_if_focus_snapshot_is_stale() {
    let mut cycle = Cycle::default();
    let snapshot = windows(&[10, 20]);
    assert_eq!(cycle.next("app", &snapshot, Some(20)), Action::Activate(10));
    assert_eq!(
        cycle.next("app", &snapshot, Some(10)),
        Action::Minimize(vec![10, 20])
    );
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(10));
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(20));
}

#[test]
fn empty_or_changed_app_starts_a_fresh_round() {
    let mut cycle = Cycle::default();
    assert_eq!(
        cycle.next("first", &windows(&[10, 20]), None),
        Action::Activate(10)
    );
    assert_eq!(
        cycle.next("second", &windows(&[30, 40]), Some(40)),
        Action::Activate(30)
    );
    assert_eq!(
        cycle.next("first", &windows(&[10, 20]), None),
        Action::Activate(10)
    );
    assert_eq!(cycle.next("first", &[], None), Action::Launch);
    assert_eq!(
        cycle.next("first", &windows(&[20, 10]), None),
        Action::Activate(20)
    );
}

#[test]
fn minimized_active_window_is_restored_instead_of_counted_as_visited() {
    let mut cycle = Cycle::default();
    let snapshot = [Window {
        id: 10,
        minimized: true,
    }];
    assert_eq!(cycle.next("app", &snapshot, Some(10)), Action::Activate(10));
}

#[test]
fn failed_activation_candidate_is_dropped_and_retried_before_progressing() {
    let mut committed = Cycle::default();
    let snapshot = windows(&[10, 20, 30]);
    assert_eq!(
        committed.next("app", &snapshot, Some(10)),
        Action::Activate(20)
    );
    let mut failed = committed.clone();
    assert_eq!(
        failed.next("app", &snapshot, Some(10)),
        Action::Activate(30)
    );
    // The native action failed. Drop the candidate instead of committing it.
    drop(failed);
    let mut retry = committed.clone();
    assert_eq!(retry.next("app", &snapshot, Some(10)), Action::Activate(30));
    committed = retry;
    assert_eq!(
        committed.next("app", &snapshot, Some(10)),
        Action::Minimize(vec![10, 20, 30])
    );
}

#[test]
fn failed_collapse_candidate_retries_collapse_instead_of_starting_a_new_round() {
    let mut committed = Cycle::default();
    let snapshot = windows(&[10]);
    assert_eq!(committed.next("app", &snapshot, None), Action::Activate(10));
    let mut failed = committed.clone();
    assert_eq!(
        failed.next("app", &snapshot, Some(10)),
        Action::Minimize(vec![10])
    );
    drop(failed);
    let mut retry = committed.clone();
    assert_eq!(
        retry.next("app", &snapshot, None),
        Action::Minimize(vec![10])
    );
    committed = retry;
    assert_eq!(
        committed.next("app", &snapshot, Some(10)),
        Action::Activate(10)
    );
}

#[test]
fn previous_round_stale_focus_does_not_skip_a_window_in_the_next_round() {
    let mut cycle = Cycle::default();
    let snapshot = windows(&[10, 20, 30]);
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(10));
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(20));
    assert_eq!(
        cycle.next("app", &snapshot, Some(30)),
        Action::Minimize(vec![10, 20, 30])
    );
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(10));
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(20));
    assert_eq!(cycle.next("app", &snapshot, Some(30)), Action::Activate(30));
    assert_eq!(
        cycle.next("app", &snapshot, Some(30)),
        Action::Minimize(vec![10, 20, 30])
    );
}
