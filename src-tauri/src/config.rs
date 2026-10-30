use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub letter: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Hint {
    pub enabled: bool,
    pub delay: f64,
    pub side: String,
    pub color: String,
    #[serde(default = "manual_order")]
    pub order: String,
}

fn manual_order() -> String {
    "manual".into()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default = "default_language")]
    pub language: String,
    pub prefix: Vec<String>,
    pub hint: Hint,
    pub apps: Vec<AppEntry>,
}

fn default_language() -> String {
    "zh-CN".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            language: default_language(),
            prefix: vec!["Control".into(), "Alt".into()],
            hint: Hint {
                enabled: true,
                delay: 0.5,
                side: "right".into(),
                color: "#71899a".into(),
                order: manual_order(),
            },
            apps: Vec::new(),
        }
    }
}

pub fn path_key(path: &str) -> String {
    #[cfg(windows)]
    {
        path.replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .to_lowercase()
    }
    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !["zh-CN", "en"].contains(&self.language.as_str()) {
            return Err("语言必须是 zh-CN 或 en".into());
        }
        if self.version != 1 {
            return Err("不支持此配置版本；原文件未修改".into());
        }
        let modifiers = ["Control", "Alt", "Shift", "Super"];
        let unique: HashSet<_> = self.prefix.iter().collect();
        if self.prefix.is_empty()
            || unique.len() != self.prefix.len()
            || self
                .prefix
                .iter()
                .any(|item| !modifiers.contains(&item.as_str()))
        {
            return Err("Prefix 必须是 Control / Alt / Shift / Super 的非空组合".into());
        }
        if !self.hint.delay.is_finite() || !(0.0..=10.0).contains(&self.hint.delay) {
            return Err("提示延迟必须在 0–10 秒之间".into());
        }
        if !["left", "right"].contains(&self.hint.side.as_str()) {
            return Err("提示位置必须是 left 或 right".into());
        }
        if !["manual", "alphabetical"].contains(&self.hint.order.as_str()) {
            return Err("提示顺序必须是 manual 或 alphabetical".into());
        }
        if self.hint.color.len() != 7
            || !self.hint.color.starts_with('#')
            || !self.hint.color[1..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err("字母背景色必须是 #RRGGBB".into());
        }
        if self.apps.len() > 500 {
            return Err("应用条目不能超过 500 个".into());
        }
        let mut ids = HashSet::new();
        let mut paths = HashSet::new();
        let mut letters = HashSet::new();
        for app in &self.apps {
            if app.id.is_empty() || app.id.len() > 100 || !ids.insert(&app.id) {
                return Err("应用 ID 为空或重复".into());
            }
            if app.name.trim().is_empty() || app.name.len() > 256 {
                return Err("应用名称不能为空或超过 256 字节".into());
            }
            if !Path::new(&app.path).is_absolute()
                || app.path.contains('\0')
                || app.path.len() > 32768
            {
                return Err(format!("{}：请填写完整应用地址", app.name));
            }
            if !paths.insert(path_key(&app.path)) {
                return Err(format!("{}：相同地址已经在列表中", app.name));
            }
            if !app.letter.is_empty()
                && (app.letter.len() != 1
                    || !app.letter.bytes().all(|byte| byte.is_ascii_uppercase())
                    || !letters.insert(&app.letter))
            {
                return Err("字母必须是唯一的 A–Z；留空表示不启用此应用".into());
            }
        }
        Ok(())
    }

    pub fn shortcuts(&self) -> Vec<(String, String)> {
        let mut prefix = self.prefix.clone();
        prefix.sort();
        self.apps
            .iter()
            .filter(|app| !app.letter.is_empty())
            .map(|app| {
                (
                    format!("{}+Key{}", prefix.join("+"), app.letter),
                    app.id.clone(),
                )
            })
            .collect()
    }
}

pub fn read(path: &Path) -> Result<Config, String> {
    if !path.exists() {
        return Ok(Config::default());
    }
    if fs::metadata(path).map_err(|error| error.to_string())?.len() > 2_000_000 {
        return Err("配置文件超过 2 MB；原文件未修改".into());
    }
    let data = fs::read(path).map_err(|error| error.to_string())?;
    let mut value: serde_json::Value = serde_json::from_slice(&data)
        .map_err(|error| format!("配置读取失败，原文件未修改：{error}"))?;
    if let Some(apps) = value.get_mut("apps").and_then(|apps| apps.as_array_mut()) {
        for app in apps {
            if let Some(app) = app.as_object_mut() {
                app.remove("args");
            }
        }
    }
    let config: Config = serde_json::from_value(value)
        .map_err(|error| format!("配置读取失败，原文件未修改：{error}"))?;
    config.validate()?;
    Ok(config)
}

pub fn write(path: &Path, config: &Config) -> Result<(), String> {
    config.validate()?;
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        let data = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|error| error.to_string())?;
        drop(file);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };
            let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let target: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    target.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temporary, path).map_err(|error| error.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
