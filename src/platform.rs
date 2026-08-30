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
