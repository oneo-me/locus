#[cfg(target_os = "macos")]
mod example {
    use gpui_kit::component::{Root, Theme, button::Button};
    use gpui_kit::{
        AppContext, Context, Entity, Render, Window, WindowBackgroundAppearance, WindowOptions,
        div, prelude::*,
    };
    use oneo_gpui_webview::{WebView, WebViewBuilder};

    struct Composition {
        webview: Entity<WebView>,
        overlay: bool,
        clicks: usize,
    }

    impl Render for Composition {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let theme = Theme::global(cx);
            div()
                .size_full()
                .flex()
                .flex_col()
                .text_color(theme.foreground)
                .child(
                    div().p_3().bg(theme.background).child(
                        Button::new("toggle-overlay")
                            .label("Toggle overlay")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.overlay = !this.overlay;
                                cx.notify();
                            })),
                    ),
                )
                .child(
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .child(self.webview.clone())
                        .when(self.overlay, |surface| {
                            surface.child(
                                div()
                                    .id("overlay")
                                    .absolute()
                                    .top_8()
                                    .left_8()
                                    .w_64()
                                    .p_4()
                                    .occlude()
                                    .bg(theme.popover.opacity(0.9))
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded(theme.radius)
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child("GPUI overlay")
                                    .child(format!("Clicks: {}", self.clicks))
                                    .child(Button::new("count").label("Count").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.clicks += 1;
                                            cx.notify();
                                        }),
                                    )),
                            )
                        }),
                )
        }
    }

    pub fn run() {
        gpui_kit::application()
            .with_assets(gpui_kit::assets::Assets)
            .run(|cx| {
                gpui_kit::init(cx);
                cx.open_window(
                    WindowOptions {
                        window_background: WindowBackgroundAppearance::Transparent,
                        ..Default::default()
                    },
                    |window, cx| {
                        let mut webview = WebView::new(
                            WebViewBuilder::new().with_html(include_str!("composition.html")),
                            window,
                            cx,
                        )
                        .unwrap();
                        webview.set_visible(true);
                        let webview = cx.new(|_| webview);
                        let view = cx.new(|_| Composition {
                            webview,
                            overlay: true,
                            clicks: 0,
                        });
                        cx.new(|cx| Root::new(view, window, cx).bg(Theme::global(cx).transparent))
                    },
                )
                .unwrap();
                cx.activate(true);
            });
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    example::run();
}
