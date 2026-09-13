use super::*;
use gpui_kit::Role;
use gpui_kit::component::{Disableable as _, button::Button};
use locus_mcp::McpServerState;
use rust_i18n::t;

impl Locus {
    pub(super) fn toggle_panel(&mut self, cx: &mut Context<Self>) {
        self.app_shell.workspace_mut().toggle(cx.reduce_motion());
        self.save_interface_state(cx);
        cx.notify();
    }

    pub(super) fn render_history_button(
        &self,
        direction: NavigationDirection,
        cx: &mut Context<Self>,
    ) -> Button {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let (id, label, icon_kind, enabled) = match direction {
            NavigationDirection::Back => (
                "navigate-back",
                t!("workspace.navigation.back", locale = language.locale()).into_owned(),
                Icon::ArrowLeft,
                self.library_navigation.can_go_back(),
            ),
            NavigationDirection::Forward => (
                "navigate-forward",
                t!("workspace.navigation.forward", locale = language.locale()).into_owned(),
                Icon::ArrowRight,
                self.library_navigation.can_go_forward(),
            ),
        };

        oneo_gpui::toolbar_icon_button(id, label, icon_kind, &theme)
            .when(!enabled, |button| button.opacity(0.35))
            .disabled(!enabled)
            .on_click(cx.listener(move |this, _, _, cx| match direction {
                NavigationDirection::Back => this.navigate_back(cx),
                NavigationDirection::Forward => this.navigate_forward(cx),
            }))
    }

    pub(super) fn render_content_titlebar(
        &self,
        left_control_clearance: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        oneo_gpui::content_titlebar(
            oneo_gpui::titlebar_drag_region("content-titlebar")
                .debug_selector(|| "content-titlebar-drag".to_owned())
                .w_full()
                .flex()
                .items_center()
                .pl(px(titlebar_content_left_padding(
                    left_control_clearance,
                    self.app_shell.workspace().rendered_width(),
                    14.0,
                )))
                .pr(px(14.0))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(
                            self.selected_library_node
                                .as_deref()
                                .and_then(|path| self.library_tree.node(path))
                                .map(|node| node.title.clone())
                                .unwrap_or_else(|| {
                                    t!(
                                        "workspace.note.no_selection",
                                        locale = self.language(cx).locale()
                                    )
                                    .into_owned()
                                }),
                        ),
                ),
        )
    }

    pub(super) fn render_mcp_status(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::global(cx).clone();
        let language = self.language(cx);
        let state = self.mcp_server.state();
        let (status, color) = match &state {
            McpServerState::Stopped => (
                t!("mcp.status.stopped", locale = language.locale()).into_owned(),
                theme.muted_foreground,
            ),
            McpServerState::Running => (
                t!("mcp.status.running", locale = language.locale()).into_owned(),
                theme.success,
            ),
            McpServerState::Failed { .. } => (
                t!("mcp.status.failed", locale = language.locale()).into_owned(),
                theme.danger,
            ),
        };
        let label = format!("MCP · {status}");
        div()
            .id("mcp-status")
            .role(Role::Status)
            .aria_label(label.clone())
            .tooltip(move |window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(label.clone()).build(window, cx)
            })
            .flex()
            .items_center()
            .justify_center()
            .size(gpui_kit::rems(1.75))
            .text_color(color)
            .child(gpui_kit::component::Icon::new(Icon::Network).size(gpui_kit::rems(0.875)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    struct ContentHarness {
        locus: gpui_kit::Entity<Locus>,
    }

    impl Render for ContentHarness {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            self.locus
                .update(cx, |locus, cx| locus.render_content(124.0, 800.0, cx))
        }
    }

    #[gpui_kit::test]
    fn no_note_titlebar_drag_region_fills_the_available_header(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                Locus::new(
                    window,
                    Preferences::default(),
                    None,
                    InterfaceState::default(),
                    cx,
                )
            });
            let content = cx.new(|_| ContentHarness { locus: view });
            gpui_kit::component::Root::new(content, window, cx)
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let bar = cx.debug_bounds("content-titlebar").unwrap();
        let drag = cx.debug_bounds("content-titlebar-drag").unwrap();
        assert_eq!(drag.left(), bar.left());
        assert_eq!(drag.top(), bar.top());
        assert_eq!(drag.size.height, px(TITLEBAR_HEIGHT));
        let right = cx
            .debug_bounds("window-controls")
            .map_or(bar.right(), |controls| controls.left());
        assert_eq!(drag.right(), right);
        assert!(drag.size.width > px(0.0));
    }
}
