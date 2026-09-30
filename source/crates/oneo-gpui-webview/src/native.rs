use std::{cell::RefCell, ffi::c_void, ptr::NonNull};

use anyhow::{Context as _, Result, bail};
use gpui_kit::{
    AnyWindowHandle, App, AppContext as _, AsyncApp, Bounds, FocusHandle, Hitbox, Pixels, Window,
    point, px,
};
use objc2::rc::Retained;
use raw_window_handle::RawWindowHandle;
use wry::WebViewExtMacOS as _;

unsafe extern "C" {
    fn oneo_webview_create(
        web: *mut c_void,
        gpui: *mut c_void,
        context: *mut c_void,
        route: extern "C" fn(*mut c_void, f64, f64, bool) -> bool,
    ) -> *mut c_void;
    fn oneo_webview_layout(
        host: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        clip_x: f64,
        clip_y: f64,
        clip_width: f64,
        clip_height: f64,
    );
    fn oneo_webview_visible(host: *mut c_void, visible: bool);
    fn oneo_webview_focus(host: *mut c_void, focused: bool);
    fn oneo_webview_destroy(host: *mut c_void);
}

struct Routing {
    app: AsyncApp,
    window: AnyWindowHandle,
    focus: FocusHandle,
    hitbox: Option<Hitbox>,
    visible: bool,
}

pub(crate) struct NativeHost {
    host: NonNull<c_void>,
    routing: Box<RefCell<Routing>>,
}

impl NativeHost {
    pub(crate) fn new(
        webview: &wry::WebView,
        focus: FocusHandle,
        window: &Window,
        cx: &App,
    ) -> Result<Self> {
        let RawWindowHandle::AppKit(handle) =
            raw_window_handle::HasWindowHandle::window_handle(window)?.as_raw()
        else {
            bail!("WebView requires an AppKit window");
        };
        let mut routing = Box::new(RefCell::new(Routing {
            app: cx.to_async(),
            window: window.window_handle(),
            focus,
            hitbox: None,
            visible: false,
        }));
        let native_webview = webview.webview();
        let host = unsafe {
            oneo_webview_create(
                Retained::as_ptr(&native_webview).cast_mut().cast(),
                handle.ns_view.as_ptr(),
                (&mut *routing as *mut RefCell<Routing>).cast(),
                route,
            )
        };
        Ok(Self {
            host: NonNull::new(host).context("WebView requires an attached GPUI view")?,
            routing,
        })
    }

    pub(crate) fn layout(&self, bounds: Bounds<Pixels>, clip: Bounds<Pixels>, hitbox: Hitbox) {
        self.routing.borrow_mut().hitbox = Some(hitbox);
        unsafe {
            oneo_webview_layout(
                self.host.as_ptr(),
                bounds.origin.x.into(),
                bounds.origin.y.into(),
                bounds.size.width.into(),
                bounds.size.height.into(),
                clip.origin.x.into(),
                clip.origin.y.into(),
                clip.size.width.into(),
                clip.size.height.into(),
            );
        }
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.routing.borrow_mut().visible = visible;
        unsafe { oneo_webview_visible(self.host.as_ptr(), visible) };
    }

    pub(crate) fn focus(&self, focused: bool) {
        unsafe { oneo_webview_focus(self.host.as_ptr(), focused) };
    }
}

impl Drop for NativeHost {
    fn drop(&mut self) {
        unsafe { oneo_webview_destroy(self.host.as_ptr()) };
    }
}

extern "C" fn route(context: *mut c_void, x: f64, y: f64, mouse_down: bool) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let routing = unsafe { &*context.cast::<RefCell<Routing>>() };
        let Ok(routing) = routing.try_borrow() else {
            return false;
        };
        if !routing.visible {
            return false;
        }
        let Some(hitbox) = routing.hitbox.clone() else {
            return false;
        };
        let focus = routing.focus.clone();
        let window = routing.window;
        let mut app = routing.app.clone();
        drop(routing);
        app.update_window(window, |_, window, cx| {
            if window
                .captured_hitbox()
                .is_some_and(|captured| captured != hitbox.id)
            {
                return false;
            }
            let position = point(px(x as f32), px(y as f32));
            if !hitbox.is_hovered_at(position, window) {
                return false;
            }
            if mouse_down {
                window.focus(&focus, cx);
            }
            true
        })
        .unwrap_or(false)
    }))
    .unwrap_or(false)
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
