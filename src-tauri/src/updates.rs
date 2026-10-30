use crate::runtime::Runtime;
use serde::Serialize;
use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

pub type PendingUpdate = tauri::async_runtime::Mutex<Option<Update>>;

#[derive(Serialize)]
pub struct UpdateInfo {
    version: String,
    notes: String,
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let state = app.state::<PendingUpdate>();
    let mut pending = state
        .try_lock()
        .map_err(|_| "更新操作正在进行，请稍后再试")?;
    let mut update = app
        .updater_builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?
        .check()
        .await
        .map_err(|error| match error {
            tauri_plugin_updater::Error::ReleaseNotFound => {
                "尚未发布可用版本，或更新清单暂不可用。".into()
            }
            error => format!("检查更新失败：{error}"),
        })?;
    let info = update.as_mut().map(|update| {
        update.timeout = Some(Duration::from_secs(300));
        UpdateInfo {
            version: update.version.clone(),
            notes: update.body.clone().unwrap_or_default(),
        }
    });
    if let Some(tray) = app.tray_by_id("main") {
        let english = app
            .state::<Arc<Runtime>>()
            .data
            .lock()
            .unwrap()
            .config
            .language
            == "en";
        let tooltip = match (&info, english) {
            (Some(info), true) => format!(
                "AppFlash · Version {} available. Open Settings to update",
                info.version
            ),
            (None, true) => "AppFlash · Your apps, in a flash.".into(),
            (Some(info), false) => format!("AppFlash · 发现新版本 {}，打开设置更新", info.version),
            (None, false) => "AppFlash · 你的应用，一闪即达。".into(),
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
    *pending = update;
    Ok(info)
}

#[tauri::command]
pub async fn install_update(app: AppHandle, version: String) -> Result<(), String> {
    let state = app.state::<PendingUpdate>();
    let mut pending = state
        .try_lock()
        .map_err(|_| "更新操作正在进行，请稍后再试")?;
    let update = pending.as_ref().ok_or("请先检查更新")?;
    if update.version != version {
        return Err("待更新版本已变化，请重新检查并确认".into());
    }
    let runtime = app.state::<Arc<Runtime>>();
    let path = runtime.config_path.clone();
    if path.exists() {
        std::fs::copy(&path, path.with_extension("pre-update.json"))
            .map_err(|error| format!("配置备份失败，取消升级：{error}"))?;
    }
    let mut received = 0u64;
    update
        .download_and_install(
            |chunk, total| {
                received += chunk as u64;
                let _ = app.emit(
                    "update-progress",
                    serde_json::json!({ "received": received, "total": total }),
                );
            },
            || {
                let _ = app.emit("update-installing", ());
            },
        )
        .await
        .map_err(|error| format!("升级失败：{error}"))?;
    *pending = None;
    #[cfg(target_os = "macos")]
    app.restart();
    Ok(())
}
