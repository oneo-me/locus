use gpui_kit::PlatformDisplay;

#[cfg(target_os = "macos")]
pub(super) fn refresh_rate(display: &dyn PlatformDisplay) -> Option<f64> {
    let id: u64 = display.id().into();
    core_graphics::display::CGDisplay::new(id as u32)
        .display_mode()
        .map(|mode| mode.refresh_rate())
}

#[cfg(target_os = "windows")]
pub(super) fn refresh_rate(display: &dyn PlatformDisplay) -> Option<f64> {
    use windows::{Win32::Graphics::Gdi::*, core::PCWSTR};

    let id: u64 = display.id().into();
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(HMONITOR(id as _), &mut info as *mut _ as *mut MONITORINFO) }
        .as_bool()
    {
        return None;
    }
    let mut mode = DEVMODEW {
        dmSize: std::mem::size_of::<DEVMODEW>() as u16,
        ..Default::default()
    };
    if !unsafe {
        EnumDisplaySettingsW(
            PCWSTR(info.szDevice.as_ptr()),
            ENUM_CURRENT_SETTINGS,
            &mut mode,
        )
    }
    .as_bool()
    {
        return None;
    }
    Some(mode.dmDisplayFrequency as f64)
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub(super) fn refresh_rate(_: &dyn PlatformDisplay) -> Option<f64> {
    None
}
