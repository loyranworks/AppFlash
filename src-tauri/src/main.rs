#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod bindings;
mod config;
mod diagnostics;
mod icon;
#[cfg(target_os = "macos")]
mod macos_cycle;
#[cfg(target_os = "macos")]
mod macos_input;
#[cfg(target_os = "macos")]
mod macos_overlay;
mod overlay_layout;
mod overlay_notice;
mod platform;
mod runtime;
mod switching;
mod updates;
use std::sync::{atomic::Ordering, Arc};
use tauri::{Emitter, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::ShortcutState;

#[tauri::command]
async fn export_logs(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<Option<String>, String> {
    if window.label() != "settings" {
        return Err("此窗口不能导出日志。".into());
    }
    let english = app
        .state::<Arc<runtime::Runtime>>()
        .data
        .lock()
        .unwrap()
        .config
        .language
        == "en";
    tauri::async_runtime::spawn_blocking(move || {
        let selected = app
            .dialog()
            .file()
            .set_parent(&window)
            .set_title(if english {
                "Export diagnostics"
            } else {
                "导出诊断日志"
            })
            .set_file_name("AppFlash-diagnostics.txt")
            .add_filter("Text", &["txt"])
            .blocking_save_file();
        let Some(selected) = selected else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|error| format!("无法读取导出位置：{error}"))?;
        diagnostics::export(&path).map_err(|error| format!("导出日志失败：{error}"))?;
        diagnostics::record("export", "TXT diagnostics exported");
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|error| format!("导出任务未完成：{error}"))?
}

#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|error| error.to_string())?;
    manager.is_enabled().map_err(|error| error.to_string())
}

#[tauri::command]
fn get_state(app: tauri::AppHandle) -> runtime::Status {
    runtime::status(&app.state::<Arc<runtime::Runtime>>())
}

#[tauri::command]
fn set_language(app: tauri::AppHandle, language: String) -> Result<(), String> {
    let runtime = app.state::<Arc<runtime::Runtime>>();
    let mut data = runtime.data.lock().map_err(|error| error.to_string())?;
    if data.unreadable {
        return Err("原配置无法读取，已禁止覆盖。请先备份并修复配置文件".into());
    }
    let mut config = data.config.clone();
    config.language = language;
    crate::config::write(&runtime.config_path, &config)?;
    data.config = config.clone();
    drop(data);
    let _ = app.emit("config-changed", &config);
    if let Err(error) = refresh_tray(&app) {
        let _ = app.emit("app-error", error.to_string());
    }
    Ok(())
}

fn refresh_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{AboutMetadata, Menu, MenuItem, PredefinedMenuItem};
    let english = app
        .state::<Arc<runtime::Runtime>>()
        .data
        .lock()
        .unwrap()
        .config
        .language
        == "en";
    let settings = MenuItem::with_id(
        app,
        "settings",
        if english { "Settings…" } else { "设置…" },
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        if english {
            "Pause / resume switching"
        } else {
            "暂停 / 恢复快捷切换"
        },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if english {
            "Quit AppFlash"
        } else {
            "退出 AppFlash"
        },
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let studio = if english {
        "loyran works.\nMake Ideas Work Beautifully"
    } else {
        "落雁工作室\n巧思成形，琢而至美。"
    };
    let about = PredefinedMenuItem::about(
        app,
        Some(if english {
            "About AppFlash"
        } else {
            "关于 AppFlash"
        }),
        Some(AboutMetadata {
            name: Some("AppFlash".into()),
            version: Some(app.package_info().version.to_string()),
            comments: Some(studio.into()),
            credits: Some(studio.into()),
            ..Default::default()
        }),
    )?;
    let menu = Menu::with_items(app, &[&settings, &pause, &separator, &about, &quit])?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(if english {
            "AppFlash · Your apps, in a flash."
        } else {
            "AppFlash · 你的应用，一闪即达。"
        }))?;
    }
    Ok(())
}

fn refresh_icon(app: &tauri::AppHandle) {
    let runtime = app.state::<Arc<runtime::Runtime>>();
    let data = runtime.data.lock().unwrap();
    let image = icon::image(&data.config.hint.color, data.paused);
    drop(data);
    let result: Result<(), String> = (|| {
        let image = image?;
        if let Some(tray) = app.tray_by_id("main") {
            tray.set_icon(Some(image.clone()))
                .map_err(|error| error.to_string())?;
        }
        if let Some(window) = app.get_webview_window("settings") {
            window
                .set_icon(image.clone())
                .map_err(|error| error.to_string())?;
            #[cfg(target_os = "windows")]
            app.run_on_main_thread(move || {
                let result = window
                    .hwnd()
                    .map_err(|error| error.to_string())
                    .and_then(|handle| {
                        window
                            .state::<icon::TaskbarIcon>()
                            .set(handle.0 as _, &image)
                    });
                if let Err(error) = result {
                    let _ = window.emit("app-error", error);
                }
            })
            .map_err(|error| error.to_string())?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = app.emit("app-error", error);
    }
}

#[tauri::command]
async fn save_config(
    app: tauri::AppHandle,
    config: config::Config,
) -> Result<runtime::Status, String> {
    tauri::async_runtime::spawn_blocking(move || runtime::save(&app, config))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn set_paused(app: tauri::AppHandle, paused: bool) -> Result<runtime::Status, String> {
    tauri::async_runtime::spawn_blocking(move || runtime::pause(&app, paused))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn list_running() -> Result<Vec<platform::RunningApp>, String> {
    tauri::async_runtime::spawn_blocking(platform::running_apps)
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn app_icon(path: String) -> Result<String, String> {
    if path.len() > 32768 || !std::path::Path::new(&path).is_absolute() {
        return Err("无效应用路径".into());
    }
    tauri::async_runtime::spawn_blocking(move || platform::icon(&path))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn request_access() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return tauri::async_runtime::spawn_blocking(platform::request_access)
        .await
        .map_err(|error| error.to_string())?;
    #[cfg(not(target_os = "macos"))]
    Ok(())
}

fn show_settings(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("settings") {
        if let Err(error) = window
            .unminimize()
            .and_then(|_| window.show())
            .and_then(|_| window.set_focus())
        {
            eprintln!("AppFlash 无法显示设置窗口：{error}");
        }
    }
}

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updates::PendingUpdate::new(None))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--background"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    runtime::hotkey(app, shortcut.id(), event.state == ShortcutState::Pressed);
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_language,
            save_config,
            set_paused,
            list_running,
            app_icon,
            request_access,
            get_autostart,
            set_autostart,
            export_logs,
            updates::check_update,
            updates::install_update
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            app.manage(icon::TaskbarIcon::default());
            runtime::start(app.handle())?;
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
            TrayIconBuilder::with_id("main")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/icon.png"
                ))?)
                .tooltip("AppFlash · 你的应用，一闪即达。")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "settings" => show_settings(app),
                    "pause" => {
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            let paused = handle
                                .state::<Arc<runtime::Runtime>>()
                                .data
                                .lock()
                                .unwrap()
                                .paused;
                            if let Err(error) = runtime::pause(&handle, !paused) {
                                let _ = handle.emit("app-error", error);
                                show_settings(&handle);
                            }
                        });
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_settings(tray.app_handle());
                    }
                })
                .build(app)?;
            refresh_tray(app.handle())?;
            refresh_icon(app.handle());
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if std::env::args().any(|argument| argument == "--background") {
                app.state::<Arc<runtime::Runtime>>()
                    .editing
                    .store(false, Ordering::SeqCst);
            } else {
                show_settings(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "settings" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                    window
                        .state::<Arc<runtime::Runtime>>()
                        .editing
                        .store(false, Ordering::SeqCst);
                }
                tauri::WindowEvent::Focused(focused) => {
                    window
                        .state::<Arc<runtime::Runtime>>()
                        .editing
                        .store(*focused, Ordering::SeqCst);
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!());
    match result {
        Ok(app) => app.run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_settings(_app);
            }
        }),
        Err(error) => eprintln!("AppFlash 启动失败：{error}"),
    }
}
