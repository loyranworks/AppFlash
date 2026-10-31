use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

/// Own one switch from the input callback until the worker has finished it.
/// Busy presses are discarded before they can enter the channel.
#[derive(Clone, Default)]
pub struct Gate(Arc<State>);

#[derive(Default)]
struct State {
    busy: AtomicBool,
    dropped: AtomicU64,
}

pub struct Permit(Arc<State>);

impl Gate {
    pub fn try_begin(&self) -> Option<Permit> {
        if self
            .0
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            Some(Permit(self.0.clone()))
        } else {
            self.0.dropped.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
    pub fn take_dropped(&self) -> u64 {
        self.0.dropped.swap(0, Ordering::Relaxed)
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::Release);
    }
}
