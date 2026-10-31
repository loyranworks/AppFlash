use super::{RunningApp, Snapshot};
use crate::{config::AppEntry, switching::Window};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Mutex, OnceLock},
};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
}

struct Helper {
    _child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
static HELPER: OnceLock<Mutex<Option<Helper>>> = OnceLock::new();

fn start_helper() -> Result<Helper, String> {
    let bytes = include_bytes!(env!("APP_FLASH_NATIVE"));
    let hash = bytes.iter().fold(14695981039346656037u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(1099511628211)
    });
    let home = std::env::var_os("HOME").ok_or("无法读取 HOME")?;
    let directory =
        std::path::PathBuf::from(home).join("Library/Application Support/local.appflash.desktop");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("native-{hash:x}"));
    if path.exists() {
        if std::fs::read(&path).map_err(|error| error.to_string())? != bytes {
            return Err("本地原生组件被修改，请删除后重试".into());
        }
    } else {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|error| error.to_string())?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| error.to_string())?;
    Ok(Helper {
        input: child.stdin.take().unwrap(),
        output: BufReader::new(child.stdout.take().unwrap()),
        _child: child,
    })
}

fn request(value: Value) -> Result<Value, String> {
    let mut slot = HELPER.get_or_init(|| Mutex::new(None)).lock().unwrap();
    if slot.is_none() {
        *slot = Some(start_helper()?);
    }
    let helper = slot.as_mut().unwrap();
    let result = (|| {
        serde_json::to_writer(&mut helper.input, &value).map_err(|error| error.to_string())?;
        helper
            .input
            .write_all(b"\n")
            .and_then(|_| helper.input.flush())
            .map_err(|error| error.to_string())?;
        let mut line = String::new();
        if helper
            .output
            .read_line(&mut line)
            .map_err(|error| error.to_string())?
            == 0
        {
            return Err("macOS 原生组件已退出，请重新尝试".into());
        }
        let result: Value = serde_json::from_str(&line).map_err(|error| error.to_string())?;
        if let Some(error) = result.get("error").and_then(Value::as_str) {
            return Err(error.to_owned());
        }
        Ok(result["data"].clone())
    })();
    if result.is_err() && helper._child.try_wait().ok().flatten().is_some() {
        *slot = None;
    }
    result
}

fn parse_snapshot(value: Value) -> Result<Snapshot, String> {
    #[derive(Deserialize)]
    struct NativeWindow {
        id: u64,
        minimized: bool,
    }
    #[derive(Deserialize)]
    struct NativeSnapshot {
        windows: Vec<NativeWindow>,
        active: Option<u64>,
    }
    let result: NativeSnapshot =
        serde_json::from_value(value).map_err(|error| error.to_string())?;
    Ok(Snapshot {
        windows: result
            .windows
            .into_iter()
            .map(|window| Window {
                id: window.id,
                minimized: window.minimized,
            })
            .collect(),
        active: result.active,
    })
}

pub fn snapshot(path: &str) -> Result<Snapshot, String> {
    parse_snapshot(request(json!({"command":"snapshot", "path":path}))?)
}

pub fn prepare_collapse(path: &str, ids: &[u64]) -> Result<Option<Snapshot>, String> {
    let result = request(json!({"command":"prepare-collapse", "path":path, "ids":ids}))?;
    if result.is_null() {
        Ok(None)
    } else {
        parse_snapshot(result).map(Some)
    }
}

pub fn running_apps() -> Result<Vec<RunningApp>, String> {
    #[derive(Deserialize)]
    struct NativeApp {
        name: String,
        path: String,
    }
    let apps: Vec<NativeApp> = serde_json::from_value(request(json!({"command":"list"}))?)
        .map_err(|error| error.to_string())?;
    Ok(apps
        .into_iter()
        .map(|app| RunningApp {
            name: app.name,
            path: app.path,
        })
        .collect())
}

pub fn launch(entry: &AppEntry) -> Result<(), String> {
    let mut command = Command::new("/usr/bin/open");
    command.args(["-a", &entry.path]);
    let result = command.output().map_err(|error| error.to_string())?;
    if result.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&result.stderr).into())
    }
}

pub fn activate(id: u64) -> Result<(), String> {
    request(json!({"command":"activate", "id":id})).map(|_| ())
}
pub fn collapse(ids: &[u64]) -> Result<(), String> {
    request(json!({"command":"collapse", "ids":ids})).map(|_| ())
}
pub fn request_access() -> Result<(), String> {
    request(json!({"command":"access"})).map(|_| ())
}
pub fn icon(path: &str) -> Result<String, String> {
    request(json!({"command":"icon", "path":path}))?
        .as_str()
        .map(str::to_owned)
        .ok_or("图标不可用".into())
}

pub fn prefix_down(prefix: &[String]) -> bool {
    [
        ("Control", 59, 62),
        ("Alt", 58, 61),
        ("Shift", 56, 60),
        ("Super", 55, 54),
    ]
    .iter()
    .all(|(name, left, right)| {
        let pressed =
            unsafe { CGEventSourceKeyState(1, *left) || CGEventSourceKeyState(1, *right) };
        pressed == prefix.iter().any(|item| item == name)
    })
}

pub fn key_down(letter: &str) -> bool {
    let codes = [
        0, 11, 8, 2, 14, 3, 5, 4, 34, 38, 40, 37, 46, 45, 31, 35, 12, 15, 1, 17, 32, 9, 13, 7, 16,
        6,
    ];
    letter
        .bytes()
        .next()
        .filter(u8::is_ascii_uppercase)
        .map(|key| unsafe { CGEventSourceKeyState(1, codes[(key - b'A') as usize]) })
        .unwrap_or(false)
}
