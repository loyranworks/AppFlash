#[cfg(target_os = "windows")]
struct NativeIcon(isize);

#[cfg(target_os = "windows")]
impl Drop for NativeIcon {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::DestroyIcon(self.0 as _);
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Default)]
pub struct TaskbarIcon(std::sync::Mutex<Option<NativeIcon>>);

#[cfg(target_os = "windows")]
impl TaskbarIcon {
    pub fn set(
        &self,
        window: windows_sys::Win32::Foundation::HWND,
        image: &tauri::image::Image<'_>,
    ) -> Result<(), String> {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateIcon, SendMessageW, ICON_BIG, WM_SETICON,
        };
        let mut pixels = image.rgba().to_vec();
        let mut mask = Vec::with_capacity(pixels.len() / 4);
        for pixel in pixels.chunks_exact_mut(4) {
            mask.push(pixel[3].wrapping_sub(255));
            pixel.swap(0, 2);
        }
        let handle = unsafe {
            CreateIcon(
                std::ptr::null_mut(),
                image.width() as i32,
                image.height() as i32,
                1,
                32,
                mask.as_ptr(),
                pixels.as_ptr(),
            )
        };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let icon = NativeIcon(handle as isize);
        let mut current = self.0.lock().map_err(|error| error.to_string())?;
        unsafe {
            SendMessageW(window, WM_SETICON, ICON_BIG as usize, icon.0);
        }
        *current = Some(icon);
        Ok(())
    }
}

pub fn image(color: &str, paused: bool) -> Result<tauri::image::Image<'static>, String> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("字母背景色必须是 #RRGGBB".into());
    }
    let color = if paused { "#343840" } else { color };
    let rgb = [
        u8::from_str_radix(&color[1..3], 16).unwrap(),
        u8::from_str_radix(&color[3..5], 16).unwrap(),
        u8::from_str_radix(&color[5..7], 16).unwrap(),
    ];
    let source = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))
        .map_err(|error| error.to_string())?;
    let mut pixels = source.rgba().to_vec();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel[..3].copy_from_slice(&rgb);
    }
    Ok(tauri::image::Image::new_owned(
        pixels,
        source.width(),
        source.height(),
    ))
}
