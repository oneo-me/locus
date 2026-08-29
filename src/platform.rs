use gpui::Window;

#[cfg(target_os = "macos")]
pub fn show_about_panel() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(main_thread) = MainThreadMarker::new() else {
        return;
    };
    NSApplication::sharedApplication(main_thread).orderFrontStandardAboutPanel(None);
}

#[cfg(not(target_os = "macos"))]
pub fn show_about_panel() {}

#[cfg(target_os = "macos")]
pub fn set_window_appearance(window: &Window, dark: Option<bool>) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAppearance, NSAppearanceCustomization, NSAppearanceNameAqua, NSAppearanceNameDarkAqua,
        NSView,
    };
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
        let Some(native_window) = view.window() else {
            return;
        };
        let appearance = dark.and_then(|dark| {
            NSAppearance::appearanceNamed(if dark {
                NSAppearanceNameDarkAqua
            } else {
                NSAppearanceNameAqua
            })
        });
        native_window.setAppearance(appearance.as_deref());
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_window_appearance(_: &Window, _: Option<bool>) {}

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

#[cfg(target_os = "macos")]
struct SidebarMaterial {
    tint: objc2::rc::Retained<objc2_app_kit::NSView>,
}

#[cfg(target_os = "macos")]
thread_local! {
    static SIDEBAR_MATERIAL: std::cell::RefCell<Option<SidebarMaterial>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(target_os = "macos")]
pub fn init_reduce_motion(cx: &mut gpui::App) {
    use objc2_app_kit::NSWorkspace;

    cx.set_reduce_motion(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion());
}

#[cfg(not(target_os = "macos"))]
pub fn init_reduce_motion(_: &mut gpui::App) {}

#[cfg(target_os = "macos")]
pub fn configure_sidebar_material(window: &Window, dark: bool) -> Result<(), &'static str> {
    use objc2::{MainThreadMarker, MainThreadOnly};
    use objc2_app_kit::{
        NSAutoresizingMaskOptions, NSColor, NSView, NSVisualEffectBlendingMode,
        NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindowOrderingMode,
    };
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    use crate::app::LEFT_SIDEBAR_WIDTH;

    let handle = HasWindowHandle::window_handle(window)
        .map_err(|_| "GPUI did not expose a native window handle")?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err("GPUI window is not backed by AppKit");
    };
    let Some(main_thread) = MainThreadMarker::new() else {
        return Err("sidebar material must be configured on the main thread");
    };

    unsafe {
        let gpui_view = handle.ns_view.cast::<NSView>().as_ref();
        let Some(native_window) = gpui_view.window() else {
            return Err("GPUI view is not attached to an NSWindow");
        };
        let background = if dark {
            NSColor::colorWithSRGBRed_green_blue_alpha(0.0, 0.0, 0.0, 0.25)
        } else {
            NSColor::colorWithSRGBRed_green_blue_alpha(1.0, 1.0, 1.0, 0.0)
        };
        native_window.setBackgroundColor(Some(&background));

        let Some(content_view) = native_window.contentView() else {
            return Err("NSWindow has no content view");
        };
        let mut configured_effect = false;
        for subview in content_view.subviews().iter() {
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

        let mut frame = content_view.bounds();
        frame.size.width = LEFT_SIDEBAR_WIDTH.into();
        let tint = NSView::initWithFrame(NSView::alloc(main_thread), frame);
        tint.setAutoresizingMask(NSAutoresizingMaskOptions::ViewHeightSizable);
        tint.setWantsLayer(true);
        content_view.addSubview_positioned_relativeTo(
            &tint,
            NSWindowOrderingMode::Below,
            Some(gpui_view),
        );
        set_tint_color(&tint, dark);

        SIDEBAR_MATERIAL.with_borrow_mut(|material| {
            *material = Some(SidebarMaterial { tint });
        });
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn set_tint_color(view: &objc2_app_kit::NSView, dark: bool) {
    use objc2_app_kit::NSColor;

    let channel = if dark { 0x18 } else { 0xf3 } as f64 / 255.0;
    let tint = NSColor::colorWithSRGBRed_green_blue_alpha(channel, channel, channel, 0.92);
    if let Some(layer) = view.layer() {
        layer.setBackgroundColor(Some(&tint.CGColor()));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn configure_sidebar_material(_: &Window, _: bool) -> Result<(), &'static str> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn set_sidebar_material(width: f32, dark: bool) -> Result<(), &'static str> {
    let Some(_main_thread) = objc2::MainThreadMarker::new() else {
        return Err("sidebar material must be resized on the main thread");
    };

    SIDEBAR_MATERIAL.with_borrow(|material| {
        let material = material
            .as_ref()
            .ok_or("sidebar material has not been installed")?;
        let mut frame = material.tint.frame();
        frame.size.width = width.into();
        material.tint.setFrame(frame);
        set_tint_color(&material.tint, dark);
        Ok(())
    })
}

#[cfg(not(target_os = "macos"))]
pub fn set_sidebar_material(_: f32, _: bool) -> Result<(), &'static str> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn set_sidebar_material_color(dark: bool) -> Result<(), &'static str> {
    let Some(_main_thread) = objc2::MainThreadMarker::new() else {
        return Err("sidebar material color must be updated on the main thread");
    };

    SIDEBAR_MATERIAL.with_borrow(|material| {
        let material = material
            .as_ref()
            .ok_or("sidebar material has not been installed")?;
        set_tint_color(&material.tint, dark);
        Ok(())
    })
}

#[cfg(not(target_os = "macos"))]
pub fn set_sidebar_material_color(_: bool) -> Result<(), &'static str> {
    Ok(())
}
