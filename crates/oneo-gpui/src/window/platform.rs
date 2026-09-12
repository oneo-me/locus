use gpui_kit::Window;

#[cfg(target_os = "macos")]
pub fn init_reduce_motion(cx: &mut gpui_kit::App) {
    use objc2_app_kit::NSWorkspace;

    cx.set_reduce_motion(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion());
}

#[cfg(not(target_os = "macos"))]
pub fn init_reduce_motion(_: &mut gpui_kit::App) {}

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
pub fn configure_main_window_close_behavior(window: &Window, cx: &gpui_kit::App) {
    window.on_window_should_close(cx, |window, cx| {
        close_main_window(window, cx);
        cfg!(debug_assertions)
    });
}

#[cfg(all(not(debug_assertions), not(target_os = "macos")))]
pub fn configure_main_window_close_behavior(_: &Window, _: &gpui_kit::App) {}

pub fn close_main_window(window: &mut Window, cx: &mut gpui_kit::App) {
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

#[cfg(target_os = "windows")]
fn windows_handle(window: &Window) -> Option<windows::Win32::Foundation::HWND> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return None;
    };
    Some(windows::Win32::Foundation::HWND(handle.hwnd.get() as *mut _))
}

#[cfg(target_os = "windows")]
pub fn configure_windows_material(window: &Window) {
    use windows::Win32::Graphics::Dwm::{
        DWMSBT_MAINWINDOW, DWMWA_SYSTEMBACKDROP_TYPE, DwmSetWindowAttribute,
    };
    let mica_active = windows_handle(window).is_some_and(|handle| {
        let backdrop = DWMSBT_MAINWINDOW.0;
        unsafe {
            DwmSetWindowAttribute(
                handle,
                DWMWA_SYSTEMBACKDROP_TYPE,
                (&backdrop as *const i32).cast(),
                std::mem::size_of_val(&backdrop) as u32,
            )
            .is_ok()
        }
    });
    if !mica_active {
        window.set_background_appearance(gpui_kit::WindowBackgroundAppearance::Blurred);
    }
}

#[cfg(target_os = "windows")]
pub fn set_windows_material_dark(window: &Window, dark: bool) {
    use windows::Win32::{
        Foundation::BOOL,
        Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute},
    };
    if let Some(handle) = windows_handle(window) {
        let dark = BOOL::from(dark);
        unsafe {
            let _ = DwmSetWindowAttribute(
                handle,
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const BOOL).cast(),
                std::mem::size_of_val(&dark) as u32,
            );
        }
    }
}

#[cfg(target_os = "windows")]
pub fn toggle_maximized(window: &Window) {
    use windows::Win32::UI::WindowsAndMessaging::{SW_MAXIMIZE, SW_RESTORE, ShowWindowAsync};
    if let Some(handle) = windows_handle(window) {
        unsafe {
            let _ = ShowWindowAsync(
                handle,
                if window.is_maximized() {
                    SW_RESTORE
                } else {
                    SW_MAXIMIZE
                },
            );
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn toggle_maximized(window: &Window) {
    window.zoom_window();
}
