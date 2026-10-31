// AppKit panel, material, and Spaces configuration for the transparent hint window.

fn corner_radius(scale: f64) -> Result<f64, String> {
    if !scale.is_finite() || !(0.5..=1.5).contains(&scale) {
        return Err("Overlay scale must be between 0.5 and 1.5".into());
    }
    Ok(24.0 * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radius_tracks_the_entire_supported_overlay_scale() {
        assert_eq!(corner_radius(0.5).unwrap(), 12.0);
        assert!((corner_radius(0.8).unwrap() - 19.2).abs() < 1e-9);
        assert_eq!(corner_radius(1.0).unwrap(), 24.0);
        assert_eq!(corner_radius(1.5).unwrap(), 36.0);
    }

    #[test]
    fn invalid_scale_never_reaches_appkit_geometry() {
        for scale in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            -1.0,
            0.0,
            0.49,
            1.51,
        ] {
            assert!(
                corner_radius(scale).is_err(),
                "accepted invalid scale {scale}"
            );
        }
    }
}

use std::cell::RefCell;

use objc2::{
    msg_send, rc::Retained, runtime::AnyClass, ClassType, MainThreadMarker, MainThreadOnly,
};
use objc2_app_kit::{
    NSAutoresizingMaskOptions, NSBackingStoreType, NSColor, NSFloatingWindowLevel, NSPanel, NSView,
    NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView,
    NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSProcessInfo, NSRect};

fn panel_style() -> NSWindowStyleMask {
    NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel
}

fn overlay_behavior(
    mut current: NSWindowCollectionBehavior,
    os_major: isize,
) -> NSWindowCollectionBehavior {
    // AppKit raises on mutually exclusive collection behaviors. Remove the
    // ordinary-window roles before applying the overlay's roles.
    current.remove(
        NSWindowCollectionBehavior::MoveToActiveSpace
            | NSWindowCollectionBehavior::Managed
            | NSWindowCollectionBehavior::Transient
            | NSWindowCollectionBehavior::ParticipatesInCycle
            | NSWindowCollectionBehavior::FullScreenPrimary
            | NSWindowCollectionBehavior::FullScreenNone
            | NSWindowCollectionBehavior::FullScreenAllowsTiling
            | NSWindowCollectionBehavior::Primary
            | NSWindowCollectionBehavior::Auxiliary
            | NSWindowCollectionBehavior::CanJoinAllApplications,
    );
    current.insert(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::Stationary
            | NSWindowCollectionBehavior::IgnoresCycle
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::FullScreenDisallowsTiling,
    );
    if os_major >= 13 {
        current.insert(NSWindowCollectionBehavior::CanJoinAllApplications);
    }
    current
}

#[cfg(test)]
mod spaces_tests {
    use super::*;

    #[test]
    fn native_panel_cannot_activate_or_gain_window_chrome() {
        let style = panel_style();
        assert!(style.contains(NSWindowStyleMask::NonactivatingPanel));
        assert!(!style.intersects(
            NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Miniaturizable
                | NSWindowStyleMask::Resizable
        ));
    }

    #[test]
    fn fullscreen_overlay_removes_conflicting_space_and_primary_flags() {
        let original = NSWindowCollectionBehavior::MoveToActiveSpace
            | NSWindowCollectionBehavior::Managed
            | NSWindowCollectionBehavior::ParticipatesInCycle
            | NSWindowCollectionBehavior::FullScreenPrimary
            | NSWindowCollectionBehavior::FullScreenNone
            | NSWindowCollectionBehavior::FullScreenAllowsTiling
            | NSWindowCollectionBehavior::Primary;
        let result = overlay_behavior(original, 26);
        assert_eq!(
            result,
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::IgnoresCycle
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::FullScreenDisallowsTiling
                | NSWindowCollectionBehavior::CanJoinAllApplications
        );
    }

    #[test]
    fn macos_12_does_not_receive_macos_13_collection_behavior() {
        let result = overlay_behavior(NSWindowCollectionBehavior::Default, 12);
        assert!(result.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(result.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(!result.contains(NSWindowCollectionBehavior::CanJoinAllApplications));
    }
}

// NSPanel and NSView are main-thread-only. Keep ownership on that same thread;
// neither raw pointers nor an unsafe Send implementation cross Tauri's runtime.
thread_local! {
    static OVERLAY: RefCell<Option<OverlayPanel>> = const { RefCell::new(None) };
}

struct OverlayPanel {
    original: Retained<NSWindow>,
    panel: Retained<NSPanel>,
    content: Retained<NSView>,
    material: Retained<NSView>,
}

impl OverlayPanel {
    fn owns(&self, original: &NSWindow) -> bool {
        std::ptr::eq(&*self.original, original)
    }

    fn sync_frame(&self) {
        // Tauri remains the geometry proxy. Its frame is already in AppKit
        // screen coordinates, including the correct monitor and scale factor.
        self.original.orderOut(None);
        self.panel.setFrame_display(self.original.frame(), false);
    }
}

impl Drop for OverlayPanel {
    fn drop(&mut self) {
        self.panel.orderOut(None);
        // Restore Wry's original view before releasing our panel. This also
        // keeps a replaced/recreated overlay from leaking a second WebView.
        if !std::ptr::eq(&*self.content, &*self.material) {
            if is_glass(&self.material) {
                // SAFETY: The public NSGlassEffectView contentView property
                // accepts nil; both retained views are still alive here.
                unsafe {
                    let _: () = msg_send![&*self.material, setContentView: None::<&NSView>];
                }
            } else {
                self.content.removeFromSuperview();
            }
        }
        self.panel.setContentView(None);
        self.original.setContentView(Some(&self.content));
        self.original.orderOut(None);
        self.panel.close();
    }
}

/// Move the existing WKWebView into a real, nonactivating AppKit panel.
///
/// All public functions in this module must run on the AppKit main thread.
/// The hidden Tao window remains Tauri's size/position proxy. Panel ownership
/// lasts until replacement or main-thread shutdown; configure is idempotent.
pub fn configure(window: &tauri::WebviewWindow) -> Result<(), String> {
    let mtm =
        MainThreadMarker::new().ok_or("Overlay configuration requires the AppKit main thread")?;
    let original = native_window(window)?;
    OVERLAY.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.as_ref().is_some_and(|overlay| overlay.owns(&original)) {
            return Ok(());
        }
        window
            .set_focusable(false)
            .map_err(|error| error.to_string())?;
        let content = original
            .contentView()
            .ok_or("Overlay has no native content view")?;
        let os_major = NSProcessInfo::processInfo()
            .operatingSystemVersion()
            .majorVersion;
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), content.bounds().size);
        // Tao's raw-window-handle obtains contentView().unwrap().window().
        // Keep a separate, lightweight view attached to its geometry proxy
        // after the real WebView moves to the panel.
        let placeholder = NSView::initWithFrame(NSView::alloc(mtm), frame);
        placeholder.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let material = if is_material(&content) {
            content.clone()
        } else {
            make_material(mtm, frame, os_major)?
        };
        apply_radius(&material, corner_radius(1.0)?)?;
        material.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        content.setAutoresizingMask(
            NSAutoresizingMaskOptions::ViewWidthSizable
                | NSAutoresizingMaskOptions::ViewHeightSizable,
        );
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            original.frame(),
            panel_style(),
            NSBackingStoreType::Buffered,
            false,
        );
        // SAFETY: Rust's Retained owns the panel. close must not release an
        // additional retain; the panel is never handed to an NSWindowController.
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setTitle(&original.title());
        panel.setOpaque(false);
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setHasShadow(true);
        panel.setIgnoresMouseEvents(true);
        panel.setHidesOnDeactivate(false);
        panel.setFloatingPanel(true);
        panel.setBecomesKeyOnlyIfNeeded(true);
        panel.setWorksWhenModal(true);
        panel.setLevel(NSFloatingWindowLevel);
        panel.setCollectionBehavior(overlay_behavior(panel.collectionBehavior(), os_major));

        // Strong references above protect every view across removal from Tao.
        // No runtime class replacement or second WKWebView is involved.
        original.orderOut(None);
        original.setContentView(Some(&placeholder));
        content.setFrame(frame);
        if !std::ptr::eq(&*content, &*material) {
            if is_glass(&material) {
                // SAFETY: Runtime availability and class checks happened in
                // make_material; contentView is its public retained property.
                unsafe {
                    let _: () = msg_send![&*material, setContentView: &*content];
                }
            } else {
                material.addSubview(&content);
            }
        }
        panel.setContentView(Some(&material));
        panel.invalidateShadow();
        *slot = Some(OverlayPanel {
            original,
            panel,
            content,
            material,
        });
        Ok(())
    })
}

/// Synchronize the panel with Tauri's logical size/position, then scale its
/// native corner radius. Accepts the settings UI's 50–150% range.
pub fn set_scale(window: &tauri::WebviewWindow, scale: f64) -> Result<(), String> {
    MainThreadMarker::new().ok_or("Overlay scaling requires the AppKit main thread")?;
    let radius = corner_radius(scale)?;
    with_overlay(window, |overlay| {
        overlay.sync_frame();
        apply_radius(&overlay.material, radius)?;
        overlay.panel.invalidateShadow();
        Ok(())
    })
}

/// Order the panel without activating AppFlash or making it the key window.
pub fn set_visible(window: &tauri::WebviewWindow, visible: bool) -> Result<(), String> {
    MainThreadMarker::new().ok_or("Overlay visibility requires the AppKit main thread")?;
    with_overlay(window, |overlay| {
        if visible {
            overlay.sync_frame();
            overlay.panel.orderFrontRegardless();
        } else {
            overlay.original.orderOut(None);
            overlay.panel.orderOut(None);
        }
        Ok(())
    })
}

fn with_overlay(
    window: &tauri::WebviewWindow,
    operation: impl FnOnce(&OverlayPanel) -> Result<(), String>,
) -> Result<(), String> {
    let original = native_window(window)?;
    OVERLAY.with(|slot| {
        let slot = slot.borrow();
        let overlay = slot
            .as_ref()
            .filter(|overlay| overlay.owns(&original))
            .ok_or("Configure the native overlay panel before using it")?;
        operation(overlay)
    })
}

fn native_window(window: &tauri::WebviewWindow) -> Result<Retained<NSWindow>, String> {
    let pointer = window.ns_window().map_err(|error| error.to_string())?;
    // SAFETY: Tauri documents ns_window as the live NSWindow pointer; callers
    // have checked AppKit's main thread. Retain for this synchronous operation.
    unsafe { Retained::retain(pointer.cast::<NSWindow>()) }
        .ok_or_else(|| "Overlay native window is null".into())
}

fn glass_class() -> Option<&'static AnyClass> {
    // Dynamic lookup intentionally avoids a hard reference to the macOS 26
    // class symbol, keeping the same binary loadable on macOS 12–25.
    AnyClass::get(c"NSGlassEffectView")
}

fn is_glass(view: &NSView) -> bool {
    glass_class().is_some_and(|class| view.isKindOfClass(class))
}

fn is_material(view: &NSView) -> bool {
    is_glass(view) || view.isKindOfClass(NSVisualEffectView::class())
}

fn make_material(
    mtm: MainThreadMarker,
    frame: NSRect,
    os_major: isize,
) -> Result<Retained<NSView>, String> {
    if let Some(class) = glass_class().filter(|_| os_major >= 26) {
        // SAFETY: The public runtime class is known to inherit NSView. alloc /
        // initWithFrame follow normal retained Objective-C ownership, and all
        // selectors below are public NSGlassEffectView APIs on macOS 26+.
        unsafe {
            let allocated: objc2::rc::Allocated<NSView> = msg_send![class, alloc];
            let glass: Retained<NSView> = msg_send![allocated, initWithFrame: frame];
            let _: () = msg_send![&*glass, setStyle: 1_isize]; // NSGlassEffectViewStyleClear
            return Ok(glass);
        }
    }
    let effect = NSVisualEffectView::initWithFrame(NSVisualEffectView::alloc(mtm), frame);
    effect.setMaterial(NSVisualEffectMaterial::HUDWindow);
    effect.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
    effect.setState(NSVisualEffectState::Active);
    effect.setWantsLayer(true);
    Ok(Retained::into_super(effect))
}

fn apply_radius(material: &NSView, radius: f64) -> Result<(), String> {
    if is_glass(material) {
        // SAFETY: is_glass verified the receiver's runtime class, and callers
        // are on the main thread. NSGlassEffectView uses a CGFloat (f64 on Mac).
        unsafe {
            let _: () = msg_send![material, setCornerRadius: radius];
        }
    } else {
        let layer = material
            .layer()
            .ok_or("Native fallback material has no backing layer")?;
        layer.setCornerRadius(radius);
        layer.setMasksToBounds(true);
    }
    Ok(())
}
