use gpui::Window;

#[cfg(target_os = "macos")]
pub fn init_reduce_motion(cx: &mut gpui::App) {
    use objc2_app_kit::NSWorkspace;

    cx.set_reduce_motion(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion());
}

#[cfg(not(target_os = "macos"))]
pub fn init_reduce_motion(_: &mut gpui::App) {}

#[cfg(target_os = "macos")]
pub fn configure_sidebar_material(window: &Window) -> Result<(), &'static str> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSColor, NSView, NSVisualEffectBlendingMode, NSVisualEffectMaterial, NSVisualEffectState,
        NSVisualEffectView,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window)
        .map_err(|_| "GPUI did not expose a native window handle")?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("GPUI window is not backed by AppKit");
    };
    let Some(_main_thread) = MainThreadMarker::new() else {
        return Err("sidebar material must be configured on the main thread");
    };

    unsafe {
        let gpui_view = handle.ns_view.cast::<NSView>().as_ref();
        let Some(native_window) = gpui_view.window() else {
            return Err("GPUI view is not attached to an NSWindow");
        };
        native_window.setBackgroundColor(Some(&NSColor::clearColor()));

        let Some(content_view) = native_window.contentView() else {
            return Err("NSWindow has no content view");
        };
        let mut configured_effect = false;
        for subview in content_view.subviews().to_vec() {
            let Some(effect_view) = subview.downcast_ref::<NSVisualEffectView>() else {
                continue;
            };
            effect_view.setMaterial(NSVisualEffectMaterial::Sidebar);
            effect_view.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
            effect_view.setState(NSVisualEffectState::Active);
            configured_effect = true;
        }
        if !configured_effect {
            return Err("GPUI did not install a full-window blur view");
        }
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn configure_sidebar_material(_: &Window) -> Result<(), &'static str> {
    Ok(())
}

#[cfg(any(debug_assertions, target_os = "macos"))]
pub fn configure_main_window_close_behavior(window: &Window, cx: &gpui::App) {
    window.on_window_should_close(cx, |window, cx| {
        close_main_window(window, cx);
        cfg!(debug_assertions)
    });
}

#[cfg(all(not(debug_assertions), not(target_os = "macos")))]
pub fn configure_main_window_close_behavior(_: &Window, _: &gpui::App) {}

pub fn close_main_window(window: &mut Window, cx: &mut gpui::App) {
    #[cfg(debug_assertions)]
    {
        let _ = window;
        cx.quit();
    }
    #[cfg(all(not(debug_assertions), target_os = "macos"))]
    {
        let _ = cx;
        hide_window(window);
    }
    #[cfg(all(not(debug_assertions), not(target_os = "macos")))]
    {
        let _ = cx;
        window.remove_window();
    }
}

#[cfg(all(not(debug_assertions), target_os = "macos"))]
fn hide_window(window: &mut Window) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let Some(_main_thread) = MainThreadMarker::new() else {
        return;
    };

    unsafe {
        let view = handle.ns_view.cast::<NSView>().as_ref();
        if let Some(native_window) = view.window() {
            native_window.orderOut(None);
        }
    }
}
