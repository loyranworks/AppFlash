use std::{fs, path::PathBuf, process::Command};

struct Workspace(PathBuf);
impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

// Compile the real callback and event variants with either platform selection.
// Tauri state lookup and the OS shortcut registry are the external boundaries;
// the admission callback, Gate, Permit and channel ownership remain production.
fn verify_admission(macos: bool) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/runtime.rs")).unwrap();
    let event = &source
        [source.find("pub enum KeyEvent {").unwrap()..source.find("\npub struct Data {").unwrap()];
    let callback =
        &source[source.find("pub fn hotkey(").unwrap()..source.find("\npub fn start(").unwrap()];
    let select = |code: &str| {
        code.replace(
            "#[cfg(target_os = \"macos\")]",
            "#[cfg(test_platform_macos)]",
        )
        .replace(
            "#[cfg(not(target_os = \"macos\"))]",
            "#[cfg(not(test_platform_macos))]",
        )
    };
    let platform = if macos { "macos" } else { "windows" };
    let temporary = Workspace(std::env::temp_dir().join(format!(
        "appflash-input-admission-{}-{platform}",
        std::process::id()
    )));
    fs::create_dir_all(&temporary.0).unwrap();
    let gate = if root.join("src/input_gate.rs").exists() {
        root.join("src/input_gate.rs")
    } else {
        root.join("src/macos_input.rs")
    };
    let mut program = format!(
        "#![allow(dead_code, unused_imports)]\n#[path = {:?}] mod gate;\nuse gate as macos_input;\nuse gate as input_gate;\n",
        gate.to_str().unwrap()
    );
    program.push_str(
        r#"
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}, mpsc::{self, Sender}};
use std::time::Instant;
struct AppHandle(Arc<Runtime>);
impl AppHandle {
    fn state<T>(&self) -> Arc<Runtime> { self.0.clone() }
}
struct Runtime {
    data: Mutex<Data>, changing: AtomicBool, settings: Settings,
    input_gate: gate::Gate, notices: Mutex<()>, sender: Sender<KeyEvent>,
}
struct Data { paused: bool, config: Config }
struct Config;
impl Config {
    fn shortcuts(&self) -> Vec<(String, String)> {
        vec![("Control+KeyA".into(), "owned-fixture".into())]
    }
}
struct Settings(AtomicBool);
impl Settings {
    fn blocks_key(&self, _: bool, _: &mut (), _: Instant) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}
struct Shortcut;
impl std::str::FromStr for Shortcut {
    type Err = ();
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value == "Control+KeyA" { Ok(Self) } else { Err(()) }
    }
}
impl Shortcut { fn id(&self) -> u32 { 7 } }
fn app() -> (AppHandle, mpsc::Receiver<KeyEvent>) {
    let (sender, receiver) = mpsc::channel();
    (AppHandle(Arc::new(Runtime {
        data: Mutex::new(Data { paused: false, config: Config }),
        changing: AtomicBool::new(false), settings: Settings(AtomicBool::new(false)),
        input_gate: gate::Gate::default(), notices: Mutex::new(()), sender,
    })), receiver)
}
"#,
    );
    program.push_str(&select(event));
    program.push_str(&select(callback));
    program.push_str(
        r#"
#[test]
fn slow_or_queued_switch_drops_presses_and_never_replays_them() {
    let (app, receiver) = app();
    hotkey(&app, 7, true);
    // The event owns the permit even before the worker dequeues it.
    for _ in 0..50 { hotkey(&app, 7, true); }
    let active = receiver.try_recv().expect("Initial action is queued");
    assert!(receiver.try_recv().is_err(), "Busy presses accumulated before execution");
    // Hold it like a blocked native backend. No extra window actions may queue.
    for _ in 0..50 { hotkey(&app, 7, true); }
    assert!(receiver.try_recv().is_err(), "Busy presses accumulated during execution");
    assert_eq!(app.0.input_gate.take_dropped(), 100);
    hotkey(&app, 7, false);
    assert!(matches!(receiver.try_recv().unwrap(), KeyEvent::Release(_)));
    drop(active);
    assert!(receiver.try_recv().is_err(), "Rejected presses must not replay");
    hotkey(&app, 7, true);
    assert!(receiver.try_recv().is_ok(), "A fresh press after completion works");
}
#[test]
fn rejected_state_and_failed_delivery_do_not_leak_the_permit() {
    let (app, receiver) = app();
    app.0.settings.0.store(true, Ordering::SeqCst);
    hotkey(&app, 7, true);
    assert!(receiver.try_recv().is_err());
    app.0.settings.0.store(false, Ordering::SeqCst);
    app.0.changing.store(true, Ordering::SeqCst);
    hotkey(&app, 7, true);
    assert!(receiver.try_recv().is_err());
    app.0.changing.store(false, Ordering::SeqCst);
    drop(receiver);
    hotkey(&app, 7, true);
    assert!(app.0.input_gate.try_begin().is_some(), "Send failure releases its permit");
}
"#,
    );
    let input = temporary.0.join("callback.rs");
    let binary = temporary.0.join(if cfg!(windows) {
        "callback.exe"
    } else {
        "callback"
    });
    fs::write(&input, program).unwrap();
    let mut rustc = Command::new("rustc");
    rustc.args(["--edition=2021", "--test"]);
    if macos {
        rustc.args(["--cfg", "test_platform_macos"]);
    }
    let output = rustc.arg(&input).arg("-o").arg(&binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{platform} callback failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn windows_callback_discards_busy_presses_without_touching_window_selection() {
    verify_admission(false);
}

#[test]
fn macos_callback_keeps_its_existing_admission_guarantee() {
    verify_admission(true);
}
