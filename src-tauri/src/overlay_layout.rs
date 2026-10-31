pub fn size(
    count: usize,
    screen_width: u32,
    screen_height: u32,
    dpi: f64,
    percent: u16,
) -> (u32, u32) {
    let ratio = f64::from(percent) / 100.0;
    let maximum = (f64::from(screen_height) - 48.0 * dpi).max(1.0);
    let minimum = (120.0 * dpi * ratio).min(maximum);
    let width = (120.0 * dpi * ratio)
        .round()
        .clamp(1.0, f64::from(screen_width).max(1.0)) as u32;
    let height = ((count as f64 * 46.0 + 64.0) * dpi * ratio)
        .round()
        .clamp(minimum, maximum) as u32;
    (width, height)
}

pub fn notice_size(
    count: usize,
    screen_width: u32,
    screen_height: u32,
    dpi: f64,
    percent: u16,
) -> (u32, u32) {
    let ratio = f64::from(percent) / 100.0;
    let maximum = (f64::from(screen_height) - 48.0 * dpi).max(1.0);
    let width = (280.0 * dpi * ratio)
        .round()
        .clamp(1.0, f64::from(screen_width).max(1.0)) as u32;
    let height = (((count as f64 * 46.0 + 64.0).max(120.0) + 104.0) * dpi * ratio)
        .round()
        .clamp(1.0, maximum) as u32;
    (width, height)
}
