//! Bounded, non-waiting producers; only this consumer touches storage.

use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

const CAPACITY: usize = 128;
const MAINTENANCE_INTERVAL: Duration = Duration::from_secs(3600);

pub(super) struct Entry {
    pub now: SystemTime,
    pub category: String,
    pub message: String,
}

impl Entry {
    pub fn new(now: SystemTime, category: &str, message: &str) -> Self {
        Self {
            now,
            category: super::single_line(category, 128),
            message: super::single_line(message, super::MESSAGE_LIMIT),
        }
    }
}

pub(super) trait Storage: Send + 'static {
    fn record(&mut self, entry: Entry) -> io::Result<()>;
    fn export(
        &mut self,
        destination: &Path,
        now: SystemTime,
        offset_minutes: i32,
        dropped: u64,
    ) -> io::Result<()>;
    fn maintenance(&mut self, now: SystemTime) -> io::Result<()>;
}

enum Command {
    Record(Entry),
    Export {
        destination: PathBuf,
        now: SystemTime,
        offset_minutes: i32,
        reply: mpsc::Sender<io::Result<()>>,
    },
    Flush(mpsc::Sender<io::Result<()>>),
}

pub(super) struct Worker {
    // Never held during storage, export, or a blocking control-channel send.
    sender: Mutex<Option<SyncSender<Command>>>,
    dropped: Arc<AtomicU64>,
}

impl Worker {
    pub fn start(storage: impl Storage) -> io::Result<Self> {
        let (sender, receiver) = mpsc::sync_channel(CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let consumer_dropped = dropped.clone();
        thread::Builder::new()
            .name("appflash-diagnostics".into())
            .spawn(move || consume(storage, receiver, consumer_dropped))?;
        Ok(Self {
            sender: Mutex::new(Some(sender)),
            dropped,
        })
    }

    pub fn record(&self, entry: Entry) {
        let accepted = self.sender.try_lock().ok().is_some_and(|sender| {
            sender
                .as_ref()
                .is_some_and(|sender| sender.try_send(Command::Record(entry)).is_ok())
        });
        if !accepted {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn sender(&self) -> io::Result<SyncSender<Command>> {
        self.sender
            .lock()
            .map_err(|_| unavailable())?
            .as_ref()
            .cloned()
            .ok_or_else(unavailable)
    }

    pub fn export(
        &self,
        destination: &Path,
        now: SystemTime,
        offset_minutes: i32,
    ) -> io::Result<()> {
        let sender = self.sender()?;
        let (reply, result) = mpsc::channel();
        sender
            .send(Command::Export {
                destination: destination.to_path_buf(),
                now,
                offset_minutes,
                reply,
            })
            .map_err(|_| unavailable())?;
        // Releasing this clone also allows shutdown to disconnect the consumer.
        drop(sender);
        result.recv().map_err(|_| unavailable())?
    }

    pub fn shutdown(&self, timeout: Duration) -> io::Result<()> {
        let sender = self.sender.lock().map_err(|_| unavailable())?.take();
        match sender {
            Some(sender) => barrier(sender, timeout),
            None => Ok(()),
        }
    }
}

// Drop closes the last producer without waiting for IO. The consumer drains all
// accepted commands and drops storage on disconnect. The static owner explicitly
// calls shutdown at application exit, since statics do not run Drop.
fn barrier(sender: SyncSender<Command>, timeout: Duration) -> io::Result<()> {
    let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "diagnostics timeout is too large",
        )
    })?;
    let (reply, result) = mpsc::channel();
    let mut command = Command::Flush(reply);
    loop {
        match sender.try_send(command) {
            Ok(()) => break,
            Err(TrySendError::Disconnected(_)) => return Err(unavailable()),
            Err(TrySendError::Full(pending)) => command = pending,
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(timed_out());
        }
        thread::park_timeout(remaining.min(Duration::from_millis(1)));
    }
    drop(sender);
    result
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .map_err(|error| match error {
            RecvTimeoutError::Timeout => timed_out(),
            RecvTimeoutError::Disconnected => unavailable(),
        })?
}

fn consume(mut storage: impl Storage, receiver: Receiver<Command>, dropped: Arc<AtomicU64>) {
    let _ = storage.maintenance(SystemTime::now());
    let mut maintained = Instant::now();
    let mut write_error: Option<io::Error> = None;
    loop {
        let command =
            receiver.recv_timeout(MAINTENANCE_INTERVAL.saturating_sub(maintained.elapsed()));
        match command {
            Ok(Command::Record(entry)) => {
                if let Err(error) = storage.record(entry) {
                    write_error = Some(error);
                }
            }
            Ok(Command::Export {
                destination,
                now,
                offset_minutes,
                reply,
            }) => {
                let result = storage.export(
                    &destination,
                    now,
                    offset_minutes,
                    dropped.load(Ordering::Relaxed),
                );
                let _ = reply.send(result);
            }
            Ok(Command::Flush(reply)) => {
                let result = match &write_error {
                    Some(error) => Err(io::Error::new(error.kind(), error.to_string())),
                    None => Ok(()),
                };
                let _ = reply.send(result);
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        // Check after commands too: a busy queue must not postpone retention.
        if maintained.elapsed() >= MAINTENANCE_INTERVAL {
            let _ = storage.maintenance(SystemTime::now());
            maintained = Instant::now();
        }
    }
}

fn unavailable() -> io::Error {
    io::Error::new(
        io::ErrorKind::BrokenPipe,
        "diagnostics worker is unavailable",
    )
}
fn timed_out() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "diagnostics flush deadline elapsed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc, Mutex};

    struct Gate {
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    impl Gate {
        fn block(self) {
            self.entered.send(()).unwrap();
            self.release.recv().unwrap();
        }
    }
    fn gate() -> (Gate, mpsc::Receiver<()>, mpsc::Sender<()>) {
        let (entered, observing) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        (
            Gate {
                entered,
                release: blocked,
            },
            observing,
            release,
        )
    }
    #[derive(Default)]
    struct Observations {
        records: Vec<Entry>,
        exports: Vec<(usize, u64)>,
        maintenance: usize,
    }
    struct ControlledStorage {
        observed: Arc<Mutex<Observations>>,
        record_gate: Option<Gate>,
        export_gate: Option<Gate>,
        finished: Option<mpsc::Sender<()>>,
        fail_writes: bool,
    }
    impl Storage for ControlledStorage {
        fn record(&mut self, entry: Entry) -> io::Result<()> {
            if let Some(gate) = self.record_gate.take() {
                gate.block();
            }
            if self.fail_writes {
                return Err(io::Error::other("storage write failed"));
            }
            self.observed.lock().unwrap().records.push(entry);
            Ok(())
        }
        fn export(&mut self, _: &Path, _: SystemTime, _: i32, dropped: u64) -> io::Result<()> {
            if let Some(gate) = self.export_gate.take() {
                gate.block();
            }
            let mut observed = self.observed.lock().unwrap();
            let length = observed.records.len();
            observed.exports.push((length, dropped));
            Ok(())
        }
        fn maintenance(&mut self, _: SystemTime) -> io::Result<()> {
            self.observed.lock().unwrap().maintenance += 1;
            Ok(())
        }
    }
    impl Drop for ControlledStorage {
        fn drop(&mut self) {
            if let Some(finished) = self.finished.take() {
                let _ = finished.send(());
            }
        }
    }
    fn storage() -> (
        ControlledStorage,
        Arc<Mutex<Observations>>,
        mpsc::Receiver<()>,
    ) {
        let observed = Arc::new(Mutex::new(Observations::default()));
        let (finished, completion) = mpsc::channel();
        (
            ControlledStorage {
                observed: observed.clone(),
                record_gate: None,
                export_gate: None,
                finished: Some(finished),
                fail_writes: false,
            },
            observed,
            completion,
        )
    }
    fn entry(number: usize) -> Entry {
        Entry {
            now: SystemTime::UNIX_EPOCH + Duration::from_secs(number as u64),
            category: "switch".into(),
            message: number.to_string(),
        }
    }
    const GUARD: Duration = Duration::from_secs(5);

    // A synchronous write, a blocking send, or an unbounded queue breaks this.
    #[test]
    fn records_return_while_storage_is_blocked_even_after_queue_capacity() {
        let (mut storage, observed, finished) = storage();
        let (blocked, entered, release) = gate();
        storage.record_gate = Some(blocked);
        let worker = Arc::new(Worker::start(storage).unwrap());
        worker.record(entry(0));
        entered.recv_timeout(GUARD).unwrap();
        let producer = worker.clone();
        let (completed, completion) = mpsc::channel();
        let producer_thread = std::thread::spawn(move || {
            for number in 1..=131 {
                producer.record(entry(number));
            }
            completed.send(()).unwrap();
        });
        let returned_without_storage = completion.recv_timeout(GUARD).is_ok();
        release.send(()).unwrap();
        producer_thread.join().unwrap();
        assert!(
            returned_without_storage,
            "producer waited for storage or queue capacity"
        );
        worker
            .export(Path::new("ignored"), SystemTime::now(), 0)
            .unwrap();
        worker.shutdown(GUARD).unwrap();
        finished.recv_timeout(GUARD).unwrap();
        let observed = observed.lock().unwrap();
        assert_eq!(observed.records.len(), 129);
        assert_eq!(observed.exports, vec![(129, 3)]);
        assert_eq!(observed.records[128].message, "128");
        assert_eq!(observed.records[0].now, SystemTime::UNIX_EPOCH);
        assert!(observed.maintenance >= 1);
    }

    // Export is deliberately stalled inside storage, not in the producer.
    #[test]
    fn blocked_export_never_blocks_records_and_drops_remain_visible() {
        let (mut storage, observed, finished) = storage();
        let (blocked, entered, release) = gate();
        storage.export_gate = Some(blocked);
        let worker = Arc::new(Worker::start(storage).unwrap());
        worker.record(entry(0));
        let exporter = worker.clone();
        let export =
            std::thread::spawn(move || exporter.export(Path::new("ignored"), SystemTime::now(), 0));
        entered.recv_timeout(GUARD).unwrap();
        let producer = worker.clone();
        let (completed, completion) = mpsc::channel();
        let records = std::thread::spawn(move || {
            for number in 1..=129 {
                producer.record(entry(number));
            }
            completed.send(()).unwrap();
        });
        let returned_without_export = completion.recv_timeout(GUARD).is_ok();
        release.send(()).unwrap();
        records.join().unwrap();
        export.join().unwrap().unwrap();
        assert!(returned_without_export, "record waited for export");
        worker
            .export(Path::new("ignored"), SystemTime::now(), 0)
            .unwrap();
        worker.shutdown(GUARD).unwrap();
        finished.recv_timeout(GUARD).unwrap();
        assert_eq!(observed.lock().unwrap().exports, vec![(1, 0), (129, 1)]);
    }

    #[test]
    fn full_queue_shutdown_has_a_deadline_and_disconnects_after_draining() {
        let (mut storage, observed, finished) = storage();
        let (blocked, entered, release) = gate();
        storage.record_gate = Some(blocked);
        let worker = Arc::new(Worker::start(storage).unwrap());
        worker.record(entry(0));
        entered.recv_timeout(GUARD).unwrap();
        for number in 1..=128 {
            worker.record(entry(number));
        }
        let exiting = worker.clone();
        let (returned, result) = mpsc::channel();
        let exit = std::thread::spawn(move || {
            let timeout = Duration::from_millis(20);
            let outcome = exiting.shutdown(timeout);
            let _ = returned.send(outcome);
        });
        let outcome = result.recv_timeout(GUARD);
        // Even rejected records must return after shutdown, without touching IO.
        if outcome.is_ok() {
            worker.record(entry(999));
        }
        release.send(()).unwrap();
        exit.join().unwrap();
        finished.recv_timeout(GUARD).unwrap();
        let shutdown = outcome.expect("exit waited for blocked storage");
        assert_eq!(shutdown.unwrap_err().kind(), io::ErrorKind::TimedOut);
        assert_eq!(observed.lock().unwrap().records.len(), 129);
    }

    #[test]
    fn shutdown_deadline_also_applies_while_export_is_stalled() {
        let (mut storage, _, finished) = storage();
        let (blocked, entered, release) = gate();
        storage.export_gate = Some(blocked);
        let worker = Arc::new(Worker::start(storage).unwrap());
        let exporter = worker.clone();
        let export =
            std::thread::spawn(move || exporter.export(Path::new("ignored"), SystemTime::now(), 0));
        entered.recv_timeout(GUARD).unwrap();
        let exiting = worker.clone();
        let (returned, result) = mpsc::channel();
        let exit = std::thread::spawn(move || {
            let _ = returned.send(exiting.shutdown(Duration::from_millis(20)));
        });
        let shutdown = result.recv_timeout(GUARD);
        release.send(()).unwrap();
        exit.join().unwrap();
        export.join().unwrap().unwrap();
        finished.recv_timeout(GUARD).unwrap();
        assert_eq!(
            shutdown
                .expect("exit waited for blocked export")
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn oversized_entries_are_bounded_before_they_reach_storage() {
        let (storage, observed, finished) = storage();
        let worker = Worker::start(storage).unwrap();
        worker.record(Entry::new(
            SystemTime::UNIX_EPOCH,
            &format!("switch\n{}", "类".repeat(10_000)),
            &format!("message\r\n{}", "界".repeat(100_000)),
        ));
        worker.shutdown(GUARD).unwrap();
        finished.recv_timeout(GUARD).unwrap();
        let observed = observed.lock().unwrap();
        let entry = &observed.records[0];
        assert!(entry.category.len() <= 128, "queued category was unbounded");
        assert!(
            entry.message.len() <= 16 * 1024,
            "queued message was unbounded"
        );
        assert!(!entry.category.contains('\n'));
        assert!(!entry.message.contains(['\r', '\n']));
        assert_eq!(entry.now, SystemTime::UNIX_EPOCH);
    }

    #[test]
    fn dropping_a_local_worker_drains_records_and_releases_its_storage() {
        let (storage, observed, finished) = storage();
        let worker = Worker::start(storage).unwrap();
        worker.record(entry(7));
        drop(worker);
        finished.recv_timeout(GUARD).unwrap();
        assert_eq!(observed.lock().unwrap().records[0].message, "7");
    }

    #[test]
    fn shutdown_reports_storage_errors_without_recursive_logging() {
        let (mut storage, observed, finished) = storage();
        storage.fail_writes = true;
        let worker = Worker::start(storage).unwrap();
        worker.record(entry(0));
        let error = worker.shutdown(GUARD).unwrap_err();
        assert_eq!(error.to_string(), "storage write failed");
        // An IO error must still close the producer and release its storage,
        // even while the caller retains the Worker.
        finished.recv_timeout(GUARD).unwrap();
        assert!(observed.lock().unwrap().records.is_empty());
    }
}
