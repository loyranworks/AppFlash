#![cfg(target_os = "windows")]
#[path = "../src/icon.rs"]
mod icon;

use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[test]
fn taskbar_slot_updates_and_owns_its_icon() {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    let window = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            class.as_ptr(),
            0,
            0,
            0,
            1,
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    };
    assert!(!window.is_null());
    let taskbar = icon::TaskbarIcon::default();
    taskbar
        .set(window, &icon::image("#81958a", false).unwrap())
        .unwrap();
    let active = unsafe { SendMessageW(window, WM_GETICON, ICON_BIG as usize, 0) };
    assert_ne!(active, 0);
    taskbar
        .set(window, &icon::image("#81958a", true).unwrap())
        .unwrap();
    let paused = unsafe { SendMessageW(window, WM_GETICON, ICON_BIG as usize, 0) };
    assert_ne!(paused, 0);
    assert_ne!(active, paused);
    assert_eq!(
        unsafe { SendMessageW(window, WM_GETICON, ICON_SMALL as usize, 0) },
        0
    );
    unsafe {
        DestroyWindow(window);
    }
}
