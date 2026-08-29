use gpui::Window;
use std::path::Path;
use std::process::Command;

use crate::preferences::EditorPreference;

pub fn open_directory(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err("目录不存在或无法访问。".to_owned());
    }

    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = Command::new("xdg-open");

    let status = command
        .arg(path)
        .status()
        .map_err(|error| format!("无法启动文件管理器：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("文件管理器退出，状态码：{status}"))
    }
}

pub fn open_path(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("文件不存在或无法访问。".to_owned());
    }

    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = Command::new("xdg-open");

    let status = command
        .arg(path)
        .status()
        .map_err(|error| format!("无法打开文件：{error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("打开文件的进程退出，状态码：{status}"))
    }
}

pub fn open_in_editor(path: &Path, editor: EditorPreference) -> Result<(), String> {
    if !path.is_file() {
        return Err("文件不存在或无法访问。".to_owned());
    }

    let status = match editor {
        EditorPreference::System => system_open_command(path),
        EditorPreference::VisualStudioCode => vscode_command(path),
    }
    .status()
    .map_err(|error| format!("无法启动编辑器：{error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("编辑器退出，状态码：{status}"))
    }
}

fn system_open_command(path: &Path) -> Command {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = Command::new("explorer.exe");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = Command::new("xdg-open");

    command.arg(path);
    command
}

fn vscode_command(path: &Path) -> Command {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.args(["-a", "Visual Studio Code"]);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = Command::new("code.cmd");
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    let mut command = Command::new("code");

    command.arg(path);
    command
}

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
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn configure_sidebar_material(_: &Window) -> Result<(), &'static str> {
    Ok(())
}
