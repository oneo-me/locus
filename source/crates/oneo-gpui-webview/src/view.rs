use std::rc::Rc;

use anyhow::Result;
use gpui_kit::{
    App, Bounds, Context, Element, ElementId, FocusHandle, Focusable, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, IntoElement, LayoutId, Pixels, Render, Size, Style,
    Subscription, Window, div, prelude::*,
};

use crate::native::NativeHost;

/// An opaque WebKit surface composited beneath GPUI, with native input and IME.
///
/// The window and ancestors covering this element must have transparent backgrounds.
/// GPUI overlays use normal paint order and must occlude pointer input where appropriate.
/// Retain the entity across frames and hide it when retained outside the element tree.
/// All methods and destruction run on the GPUI main thread.
pub struct WebView {
    native: Rc<NativeHost>,
    webview: wry::WebView,
    focus: FocusHandle,
    visible: bool,
    _subscriptions: Vec<Subscription>,
}

impl WebView {
    /// Creates a hidden WebView. The caller supplies navigation, IPC and page policy.
    pub fn new(
        builder: wry::WebViewBuilder<'_>,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self> {
        let webview = builder
            .with_transparent(false)
            .with_visible(false)
            .with_focused(false)
            .build_as_child(window)?;
        let focus = cx.focus_handle();
        let native = Rc::new(NativeHost::new(&webview, focus.clone(), window, cx)?);
        let focus_in = native.clone();
        let focus_out = native.clone();
        let subscriptions = vec![
            window.on_focus_in(&focus, cx, move |_, _| focus_in.focus(true)),
            window.on_focus_out(&focus, cx, move |_, _, _| focus_out.focus(false)),
        ];
        Ok(Self {
            native,
            webview,
            focus,
            visible: false,
            _subscriptions: subscriptions,
        })
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Changes native visibility immediately. Notify the owning entity after changing it.
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.native.set_visible(visible);
        }
    }

    pub fn evaluate_script(&self, script: &str) -> Result<()> {
        Ok(self.webview.evaluate_script(script)?)
    }

    pub fn reload(&self) -> Result<()> {
        Ok(self.webview.reload()?)
    }
}

impl Drop for WebView {
    fn drop(&mut self) {
        self.native.set_visible(false);
    }
}

impl Focusable for WebView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for WebView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.focus)
            .child(WebViewElement {
                native: self.native.clone(),
                visible: self.visible,
            })
    }
}

struct WebViewElement {
    native: Rc<NativeHost>,
    visible: bool,
}

impl IntoElement for WebViewElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for WebViewElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (
            window.request_layout(
                Style {
                    size: Size::full(),
                    ..Default::default()
                },
                [],
                cx,
            ),
            (),
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) -> Option<Hitbox> {
        self.visible
            .then(|| window.insert_hitbox(bounds, HitboxBehavior::BlockMouse))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        hitbox: &mut Option<Hitbox>,
        window: &mut Window,
        _: &mut App,
    ) {
        if let Some(hitbox) = hitbox {
            let clip = bounds.intersect(&window.content_mask().bounds);
            self.native.layout(bounds, clip, hitbox.clone());
        }
    }
}
