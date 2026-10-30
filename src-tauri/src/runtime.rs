use crate::{
    bindings::{self, Registry},
    config::{self, Config},
    platform,
    switching::{Action, Cycle},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct Runtime {
    pub data: Mutex<Data>,
    pub changing: AtomicBool,
    pub editing: AtomicBool,
    pub sender: Sender<KeyEvent>,
    pub config_path: PathBuf,
    pub _instance: File,
}

pub enum KeyEvent {
    Press(String),
    Release(String),
}

pub struct Data {
    pub config: Config,
    pub registered: Vec<String>,
    pub paused: bool,
    pub startup_error: Option<String>,
    pub unreadable: bool,
}

#[derive(Serialize)]
pub struct Status {
    pub config: Config,
    pub paused: bool,
    pub error: Option<String>,
    pub config_path: String,
    pub platform: String,
}

struct NativeRegistry<'a>(&'a AppHandle);
impl Registry for NativeRegistry<'_> {
    fn register(&self, key: &str) -> Result<(), String> {
        self.0
            .global_shortcut()
            .register(key)
            .map_err(|error| error.to_string())
    }
    fn unregister(&self, key: &str) -> Result<(), String> {
        self.0
            .global_shortcut()
            .unregister(key)
            .map_err(|error| error.to_string())
    }
}

pub fn status(runtime: &Runtime) -> Status {
    let data = runtime.data.lock().unwrap();
    Status {
        config: data.config.clone(),
        paused: data.paused,
        error: data.startup_error.clone(),
        config_path: runtime.config_path.to_string_lossy().into(),
        platform: std::env::consts::OS.into(),
    }
}

pub fn save(app: &AppHandle, config: Config) -> Result<Status, String> {
    config.validate()?;
    for entry in &config.apps {
        if !entry.letter.is_empty() && !std::path::Path::new(&entry.path).exists() {
            return Err(format!(
                "{}：地址不存在，请修正地址或清空字母暂时停用",
                entry.name
            ));
        }
    }
    let runtime = app.state::<Arc<Runtime>>();
    if runtime.changing.swap(true, Ordering::SeqCst) {
        return Err("正在更新绑定，请稍后再试".into());
    }
    let result: Result<(), String> = (|| {
        let mut data = runtime.data.lock().unwrap();
        if data.unreadable {
            return Err("原配置无法读取，已禁止覆盖。请先备份并修复配置文件".into());
        }
        let keys = if data.paused {
            Vec::new()
        } else {
            config.shortcuts().into_iter().map(|item| item.0).collect()
        };
        bindings::transition(&data.registered, &keys, &NativeRegistry(app), || {
            config::write(&runtime.config_path, &config)
        })?;
        data.config = config.clone();
        data.registered = keys;
        data.startup_error = None;
        Ok(())
    })();
    runtime.changing.store(false, Ordering::SeqCst);
    result?;
    let _ = app.emit("config-changed", &config);
    crate::refresh_icon(app);
    Ok(status(&runtime))
}

pub fn pause(app: &AppHandle, paused: bool) -> Result<Status, String> {
    let runtime = app.state::<Arc<Runtime>>();
    if runtime.changing.swap(true, Ordering::SeqCst) {
        return Err("正在更新绑定，请稍后再试".into());
    }
    let result: Result<(), String> = (|| {
        let mut data = runtime.data.lock().unwrap();
        let keys = if paused {
            Vec::new()
        } else {
            data.config
                .shortcuts()
                .into_iter()
                .map(|item| item.0)
                .collect()
        };
        bindings::transition(&data.registered, &keys, &NativeRegistry(app), || Ok(()))?;
        data.registered = keys;
        data.paused = paused;
        Ok(())
    })();
    runtime.changing.store(false, Ordering::SeqCst);
    result?;
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.hide();
    }
    let _ = app.emit("paused-changed", paused);
    crate::refresh_icon(app);
    Ok(status(&runtime))
}

pub fn hotkey(app: &AppHandle, id: u32, pressed: bool) {
    let runtime = app.state::<Arc<Runtime>>();
    if runtime.changing.load(Ordering::SeqCst) || runtime.editing.load(Ordering::SeqCst) {
        return;
    }
    let data = runtime.data.lock().unwrap();
    if data.paused {
        return;
    }
    if let Some((_, app_id)) = data.config.shortcuts().into_iter().find(|(key, _)| {
        key.parse::<Shortcut>()
            .map(|key| key.id() == id)
            .unwrap_or(false)
    }) {
        let _ = runtime.sender.send(if pressed {
            KeyEvent::Press(app_id)
        } else {
            KeyEvent::Release(app_id)
        });
    }
}

pub fn start(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let directory = app.path().app_config_dir()?;
    std::fs::create_dir_all(&directory)?;
    let mut options = std::fs::OpenOptions::new();
    options.create(true).read(true).write(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let instance = options.open(directory.join("instance.lock"))?;
    #[cfg(target_os = "macos")]
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(instance.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("AppFlash 已经在运行".into());
        }
    }
    let path = directory.join("config.json");
    let loaded = config::read(&path);
    let unreadable = loaded.is_err();
    let error = loaded.as_ref().err().cloned();
    let config = loaded.unwrap_or_default();
    let (sender, receiver) = mpsc::channel();
    let runtime = Arc::new(Runtime {
        data: Mutex::new(Data {
            config: config.clone(),
            registered: Vec::new(),
            paused: unreadable,
            startup_error: error,
            unreadable,
        }),
        changing: AtomicBool::new(false),
        editing: AtomicBool::new(true),
        sender,
        config_path: path,
        _instance: instance,
    });
    app.manage(runtime.clone());
    if !unreadable {
        if let Err(error) = pause(app, false) {
            let mut data = runtime.data.lock().unwrap();
            data.paused = true;
            data.startup_error = Some(error);
        }
    }
    let overlay = app.get_webview_window("overlay").ok_or("提示窗口未创建")?;
    overlay.set_ignore_cursor_events(true)?;
    overlay.set_focusable(false)?;
    #[cfg(windows)]
    if let Err(error) = window_vibrancy::apply_blur(&overlay, Some((232, 239, 242, 110))) {
        eprintln!("贴边磨砂效果不可用：{error}");
    }
    #[cfg(target_os = "macos")]
    if let Err(error) = window_vibrancy::apply_vibrancy(
        &overlay,
        window_vibrancy::NSVisualEffectMaterial::Sidebar,
        Some(window_vibrancy::NSVisualEffectState::Active),
        Some(16.0),
    ) {
        eprintln!("贴边磨砂效果不可用：{error}");
    }
    let handle = app.clone();
    let worker = runtime.clone();
    std::thread::spawn(move || {
        let mut cycle = Cycle::default();
        let mut held = HashSet::new();
        let mut launching: HashMap<String, Instant> = HashMap::new();
        loop {
            let event = match receiver.recv_timeout(Duration::from_millis(25)) {
                Ok(event) => event,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    held.retain(|letter: &String| platform::key_down(letter));
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            };
            let id = match event {
                KeyEvent::Press(id) => id,
                KeyEvent::Release(id) => {
                    let data = worker.data.lock().unwrap();
                    if let Some(entry) = data.config.apps.iter().find(|entry| entry.id == id) {
                        held.remove(&entry.letter);
                    }
                    continue;
                }
            };
            let data = worker.data.lock().unwrap();
            let config = data.config.clone();
            let paused = data.paused;
            drop(data);
            held.retain(|letter: &String| platform::key_down(letter));
            if paused
                || worker.changing.load(Ordering::SeqCst)
                || worker.editing.load(Ordering::SeqCst)
            {
                continue;
            }
            let Some(entry) = config.apps.iter().find(|entry| entry.id == id) else {
                continue;
            };
            #[cfg(target_os = "macos")]
            if !held.insert(entry.letter.clone()) {
                continue;
            }
            let _ = handle.emit("app-hit", &id);
            let result = platform::snapshot(&entry.path).and_then(|snapshot| {
                let action = cycle.next(&id, &snapshot.windows, snapshot.active);
                if action == Action::Launch {
                    if launching
                        .get(&id)
                        .map(|time| time.elapsed() < Duration::from_secs(3))
                        .unwrap_or(false)
                    {
                        return Ok(());
                    }
                    launching.insert(id.clone(), Instant::now());
                } else {
                    launching.remove(&id);
                }
                platform::execute(entry, action)
            });
            if let Err(error) = result {
                launching.remove(&id);
                let _ = handle.emit("app-error", &error);
                if let Some(tray) = handle.tray_by_id("main") {
                    let _ = tray.set_tooltip(Some(format!("AppFlash：{error}")));
                }
                eprintln!("{error}");
            }
        }
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        let mut began: Option<Instant> = None;
        let mut visible = false;
        loop {
            std::thread::sleep(Duration::from_millis(25));
            let data = runtime.data.lock().unwrap();
            let hint = data.config.hint.clone();
            let prefix = data.config.prefix.clone();
            let paused = data.paused;
            drop(data);
            let pressed = !paused
                && !runtime.changing.load(Ordering::SeqCst)
                && !runtime.editing.load(Ordering::SeqCst)
                && hint.enabled
                && platform::prefix_down(&prefix);
            if !pressed {
                began = None;
                if visible {
                    let _ = overlay.hide();
                    let _ = handle.emit_to("overlay", "hint-hide", ());
                    visible = false;
                }
                continue;
            }
            let started = *began.get_or_insert_with(Instant::now);
            if !visible && started.elapsed().as_secs_f64() >= hint.delay {
                let config = runtime.data.lock().unwrap().config.clone();
                if let Err(error) = position_hint(&handle, &config) {
                    let _ = handle.emit("app-error", error);
                    continue;
                }
                let _ = handle.emit_to("overlay", "hint-show", ());
                if overlay.show().is_ok() {
                    visible = true;
                }
            }
        }
    });
    Ok(())
}

fn position_hint(app: &AppHandle, config: &Config) -> Result<(), String> {
    let overlay = app.get_webview_window("overlay").ok_or("提示窗口不存在")?;
    let cursor = app.cursor_position().map_err(|error| error.to_string())?;
    let monitors = app
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let monitor = monitors
        .iter()
        .find(|monitor| {
            let position = monitor.position();
            let size = monitor.size();
            cursor.x >= position.x as f64
                && cursor.y >= position.y as f64
                && cursor.x < position.x as f64 + size.width as f64
                && cursor.y < position.y as f64 + size.height as f64
        })
        .or_else(|| monitors.first())
        .ok_or("没有可用屏幕")?;
    let scale = monitor.scale_factor();
    let width = (120.0 * scale).round() as u32;
    let count = config
        .apps
        .iter()
        .filter(|app| !app.letter.is_empty())
        .count();
    let height = ((count as f64 * 46.0 + 64.0) * scale)
        .round()
        .min(monitor.size().height as f64 - 48.0 * scale)
        .max(120.0 * scale) as u32;
    let x = monitor.position().x
        + if config.hint.side == "right" {
            monitor.size().width as i32 - width as i32
        } else {
            0
        };
    let y = monitor.position().y + (monitor.size().height as i32 - height as i32) / 2;
    overlay
        .set_size(PhysicalSize::new(width, height))
        .map_err(|error| error.to_string())?;
    overlay
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|error| error.to_string())
}
