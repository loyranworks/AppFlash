#[path = "../src/macos_input.rs"]
mod macos_input;
use macos_input::Gate;

#[test]
fn presses_during_a_blocked_switch_are_dropped_instead_of_replayed() {
    let gate = Gate::default();
    let first = gate.try_begin().expect("First press is accepted");
    for _ in 0..100 {
        assert!(
            gate.try_begin().is_none(),
            "A busy press must never enter the queue"
        );
    }
    assert_eq!(gate.take_dropped(), 100);
    assert_eq!(gate.take_dropped(), 0);
    drop(first);
    let fresh = gate
        .try_begin()
        .expect("Fresh press after completion works");
    assert!(gate.try_begin().is_none());
    drop(fresh);
}

#[test]
fn cancelled_send_or_failed_action_releases_the_gate() {
    let gate = Gate::default();
    let (sender, receiver) = std::sync::mpsc::channel();
    drop(receiver);
    assert!(sender.send(gate.try_begin().unwrap()).is_err());
    // SendError owns and drops its rejected event, just like the real callback.
    let failed = gate.try_begin().unwrap();
    drop(failed);
    assert!(gate.try_begin().is_some());
}

#[test]
fn concurrent_sources_cannot_enqueue_a_second_switch() {
    let gate = Gate::default();
    let first = gate.try_begin().unwrap();
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let gate = gate.clone();
            std::thread::spawn(move || assert!(gate.try_begin().is_none()))
        })
        .collect();
    for thread in threads {
        thread.join().unwrap();
    }
    drop(first);
    assert!(gate.try_begin().is_some());
}
