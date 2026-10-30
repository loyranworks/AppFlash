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
