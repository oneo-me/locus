use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    App, Context, Focusable, HitboxBehavior, IntoElement, Modifiers, Render, TestAppContext,
    Window, canvas, div, point, prelude::*, px,
};

use super::{Routing, route};

struct RoutingPreview {
    routing: Rc<RefCell<Routing>>,
    overlay: bool,
    clicks: usize,
}

impl Render for RoutingPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let routing = self.routing.clone();
        div()
            .size_full()
            .track_focus(&self.routing.borrow().focus)
            .child(
                div().w(px(200.)).h(px(100.)).overflow_hidden().child(
                    canvas(
                        move |bounds, window, _| {
                            routing.borrow_mut().hitbox =
                                Some(window.insert_hitbox(bounds, HitboxBehavior::BlockMouse));
                        },
                        |_, _, _, _| {},
                    )
                    .w(px(200.))
                    .h(px(200.)),
                ),
            )
            .when(self.overlay, |root| {
                root.child(
                    div()
                        .id("overlay")
                        .absolute()
                        .top_0()
                        .left_0()
                        .w(px(50.))
                        .h(px(50.))
                        .occlude()
                        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                            this.overlay = false;
                            cx.notify();
                        }))
                        .child(
                            gpui_kit::base::Button::new("overlay-button")
                                .w_full()
                                .h_full()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.clicks += 1;
                                    cx.notify();
                                })),
                        ),
                )
            })
    }
}

impl Focusable for RoutingPreview {
    fn focus_handle(&self, _: &App) -> gpui_kit::FocusHandle {
        self.routing.borrow().focus.clone()
    }
}

#[gpui_kit::test]
fn routes_native_input_only_to_uncovered_visible_content(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| RoutingPreview {
        routing: Rc::new(RefCell::new(Routing {
            app: cx.to_async(),
            window: window.window_handle(),
            focus: cx.focus_handle(),
            hitbox: None,
            visible: true,
        })),
        overlay: true,
        clicks: 0,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let routing = view.read_with(cx, |view, _| view.routing.clone());
    let pointer = Rc::as_ptr(&routing).cast_mut().cast();
    assert!(!route(pointer, 25., 25., true));
    cx.simulate_click(point(px(25.), px(25.)), Modifiers::default());
    assert_eq!(view.read_with(cx, |view, _| view.clicks), 1);
    assert!(route(pointer, 75., 75., true));
    assert!(!route(pointer, 75., 150., true));
    assert!(!route(pointer, 250., 75., true));
    cx.update(|window, _| assert!(routing.borrow().focus.is_focused(window)));

    assert!(view.read_with(cx, |view, _| view.overlay));
    cx.simulate_click(point(px(75.), px(75.)), Modifiers::default());
    assert!(!view.read_with(cx, |view, _| view.overlay));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(route(pointer, 25., 25., false));
    routing.borrow_mut().visible = false;
    assert!(!route(pointer, 75., 75., true));
    routing.borrow_mut().visible = true;
    routing.borrow_mut().hitbox = None;
    assert!(!route(pointer, 75., 75., true));
}
