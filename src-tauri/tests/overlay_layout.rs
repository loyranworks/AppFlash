#[path = "../src/overlay_layout.rs"]
mod overlay_layout;

#[test]
fn entire_overlay_size_uses_percent_and_display_density_once() {
    assert_eq!(overlay_layout::size(4, 1920, 1080, 1.0, 100), (120, 248));
    assert_eq!(overlay_layout::size(4, 1920, 1080, 1.0, 80), (96, 198));
    assert_eq!(overlay_layout::size(4, 3840, 2160, 2.0, 80), (192, 397));
    assert_eq!(overlay_layout::size(4, 1920, 1080, 1.0, 150), (180, 372));
}

#[test]
fn scaled_overlay_stays_inside_small_screens_and_retains_empty_state_height() {
    assert_eq!(overlay_layout::size(0, 1920, 1080, 1.0, 50), (60, 60));
    let (width, height) = overlay_layout::size(26, 640, 480, 1.0, 150);
    assert!(width <= 640 && height <= 432);
    let (width, height) = overlay_layout::size(26, 40, 30, 2.0, 150);
    assert!((1..=40).contains(&width) && (1..=30).contains(&height));
}

#[test]
fn error_layout_preserves_rows_and_scales_the_additional_notice_space_once() {
    for percent in [50, 80, 100, 150] {
        let normal = overlay_layout::size(4, 1920, 1080, 1.0, percent);
        let notice = overlay_layout::notice_size(4, 1920, 1080, 1.0, percent);
        assert_eq!(notice.0, (280.0 * percent as f64 / 100.0).round() as u32);
        assert_eq!(
            notice.1,
            ((248.0 + 104.0) * percent as f64 / 100.0).round() as u32
        );
        assert!(notice.1 > normal.1);
    }
    let (width, height) = overlay_layout::notice_size(26, 640, 480, 2.0, 150);
    assert!(width <= 640 && height <= 384);
}
