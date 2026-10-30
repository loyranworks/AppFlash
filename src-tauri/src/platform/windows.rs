use super::{RunningApp, Snapshot};
use crate::{
    config::{path_key, AppEntry},
    switching::Window,
};
use base64::Engine;
use std::{collections::HashSet, ffi::OsStr, os::windows::ffi::OsStrExt, ptr};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::{
        Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED},
        Gdi::*,
    },
    System::Threading::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

struct NativeWindow {
    id: u64,
    path: String,
    title: String,
    minimized: bool,
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> windows_sys::core::BOOL {
    if IsWindowVisible(hwnd) == 0
        || GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_TOOLWINDOW as isize != 0
    {
        return 1;
    }
    let mut cloaked: u32 = 0;
    if DwmGetWindowAttribute(
        hwnd,
        DWMWA_CLOAKED as u32,
        &mut cloaked as *mut _ as *mut _,
        4,
    ) >= 0
        && cloaked != 0
    {
        return 1;
    }
    let mut class = [0u16; 256];
    let class_length = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
    if [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
    ]
    .contains(&String::from_utf16_lossy(&class[..class_length as usize]).as_str())
    {
        return 1;
    }
    let length = GetWindowTextLengthW(hwnd);
    if length == 0 {
        return 1;
    }
    let mut title = vec![0u16; length as usize + 1];
    let written = GetWindowTextW(hwnd, title.as_mut_ptr(), title.len() as i32);
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == std::process::id() {
        return 1;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if process.is_null() {
        return 1;
    }
    let mut buffer = vec![0u16; 32768];
    let mut size = buffer.len() as u32;
    let success = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size);
    CloseHandle(process);
    if success != 0 {
        let windows = &mut *(data as *mut Vec<NativeWindow>);
        windows.push(NativeWindow {
            id: hwnd as u64,
            path: String::from_utf16_lossy(&buffer[..size as usize]),
            title: String::from_utf16_lossy(&title[..written as usize]),
            minimized: IsIconic(hwnd) != 0,
        });
    }
    1
}

fn enumerate() -> Result<Vec<NativeWindow>, String> {
    let mut windows = Vec::new();
    if unsafe { EnumWindows(Some(visit), &mut windows as *mut _ as isize) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(windows)
}

pub fn snapshot(path: &str) -> Result<Snapshot, String> {
    let key = path_key(path);
    let windows = enumerate()?
        .into_iter()
        .filter(|window| path_key(&window.path) == key)
        .map(|window| Window {
            id: window.id,
            minimized: window.minimized,
        })
        .collect();
    Ok(Snapshot {
        windows,
        active: Some(unsafe { GetForegroundWindow() } as u64),
    })
}

pub fn running_apps() -> Result<Vec<RunningApp>, String> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for window in enumerate()? {
        if seen.insert(path_key(&window.path)) {
            let name = std::path::Path::new(&window.path)
                .file_stem()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or(window.title);
            apps.push(RunningApp {
                name,
                path: window.path,
            });
        }
    }
    apps.sort_by_key(|app| app.name.to_lowercase());
    Ok(apps)
}

pub fn launch(entry: &AppEntry) -> Result<(), String> {
    std::process::Command::new(&entry.path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("启动 {} 失败：{error}", entry.name))
}

pub fn activate(id: u64) -> Result<(), String> {
    let hwnd = id as HWND;
    unsafe {
        if IsWindow(hwnd) == 0 {
            return Err("目标窗口已关闭".into());
        }
        if IsIconic(hwnd) != 0 {
            ShowWindowAsync(hwnd, SW_RESTORE);
        }
        if SetForegroundWindow(hwnd) == 0 {
            let foreground = GetForegroundWindow();
            let thread = GetWindowThreadProcessId(foreground, ptr::null_mut());
            let current = GetCurrentThreadId();
            let attached =
                thread != 0 && thread != current && AttachThreadInput(current, thread, 1) != 0;
            let result = SetForegroundWindow(hwnd);
            if attached {
                AttachThreadInput(current, thread, 0);
            }
            if result == 0 {
                return Err(
                    "系统拒绝激活窗口。目标若以管理员权限运行，请以相同权限运行 AppFlash".into(),
                );
            }
        }
    }
    Ok(())
}

pub fn minimize(id: u64) -> Result<(), String> {
    let hwnd = id as HWND;
    if unsafe { IsWindow(hwnd) } == 0 {
        return Err("目标窗口已关闭".into());
    }
    if unsafe { ShowWindowAsync(hwnd, SW_MINIMIZE) } == 0 {
        return Err("系统拒绝最小化窗口".into());
    }
    Ok(())
}

pub fn key_down(letter: &str) -> bool {
    letter
        .bytes()
        .next()
        .map(|key| unsafe { GetAsyncKeyState(key as i32) < 0 })
        .unwrap_or(false)
}

pub fn prefix_down(prefix: &[String]) -> bool {
    let keys = [
        ("Control", VK_CONTROL),
        ("Alt", VK_MENU),
        ("Shift", VK_SHIFT),
        ("Super", VK_LWIN),
    ];
    keys.iter().all(|(name, key)| {
        let pressed = unsafe {
            GetAsyncKeyState(*key as i32) < 0
                || (*name == "Super" && GetAsyncKeyState(VK_RWIN as i32) < 0)
        };
        pressed == prefix.iter().any(|item| item == name)
    })
}

pub fn icon(path: &str) -> Result<String, String> {
    unsafe {
        let mut handle = ptr::null_mut();
        let extracted = PrivateExtractIconsW(
            wide(path).as_ptr(),
            0,
            96,
            96,
            &mut handle,
            ptr::null_mut(),
            1,
            0,
        );
        if extracted == 0 || extracted == u32::MAX || handle.is_null() {
            return Err("无可用图标".into());
        }
        let screen = GetDC(ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 96;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        let mut pixels = ptr::null_mut();
        let bitmap = CreateDIBSection(
            screen,
            &info,
            DIB_RGB_COLORS,
            &mut pixels,
            ptr::null_mut(),
            0,
        );
        if bitmap.is_null() || dc.is_null() || pixels.is_null() {
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            if !dc.is_null() {
                DeleteDC(dc);
            }
            ReleaseDC(ptr::null_mut(), screen);
            DestroyIcon(handle);
            return Err("图标位图创建失败".into());
        }
        let original = SelectObject(dc, bitmap);
        ptr::write_bytes(pixels, 0, 96 * 96 * 4);
        let success = DrawIconEx(dc, 0, 0, handle, 96, 96, 0, ptr::null_mut(), DI_NORMAL);
        let mut rgba = std::slice::from_raw_parts(pixels as *const u8, 96 * 96 * 4).to_vec();
        SelectObject(dc, original);
        DeleteObject(bitmap);
        DeleteDC(dc);
        ReleaseDC(ptr::null_mut(), screen);
        DestroyIcon(handle);
        if success == 0 {
            return Err("图标绘制失败".into());
        }
        let has_alpha = rgba.chunks_exact(4).any(|pixel| pixel[3] != 0);
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
            if has_alpha && pixel[3] > 0 {
                for index in 0..3 {
                    pixel[index] = ((pixel[index] as u32 * 255 / pixel[3] as u32).min(255)) as u8;
                }
            } else if !has_alpha {
                pixel[3] = 255;
            }
        }
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 96, 96);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder
                .write_header()
                .map_err(|error| error.to_string())?
                .write_image_data(&rgba)
                .map_err(|error| error.to_string())?;
        }
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes)
        ))
    }
}
