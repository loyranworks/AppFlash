#[path = "../src/icon.rs"]
mod icon;

#[test]
fn user_color_and_pause_share_the_same_flash_shape() {
    let active = icon::image("#81958a", false).unwrap();
    let paused = icon::image("#81958a", true).unwrap();
    let resumed = icon::image("#81958a", false).unwrap();
    assert_eq!(active.rgba(), resumed.rgba());
    assert_eq!(active.width(), paused.width());
    assert_eq!(active.height(), paused.height());
    let center = ((active.height() / 2 * active.width() + active.width() / 2) * 4 + 3) as usize;
    assert_eq!(active.rgba()[center], 0);
    assert!(active.rgba().chunks_exact(4).any(|pixel| pixel[3] == 255));
    for (active_pixel, paused_pixel) in active
        .rgba()
        .chunks_exact(4)
        .zip(paused.rgba().chunks_exact(4))
    {
        assert_eq!(active_pixel[3], paused_pixel[3]);
        if active_pixel[3] != 0 {
            assert_eq!(&active_pixel[..3], &[0x81, 0x95, 0x8a]);
            assert_eq!(&paused_pixel[..3], &[0x34, 0x38, 0x40]);
        }
    }
}

#[test]
fn invalid_colors_are_rejected_without_panicking() {
    for color in ["", "blue", "#123", "#zzzzzz", "#éabc"] {
        assert!(icon::image(color, false).is_err());
    }
}
