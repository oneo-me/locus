use super::*;

impl Settings {
    fn toggle_dropdown(&mut self, dropdown: Dropdown, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_dropdown == Some(dropdown) {
            self.open_dropdown = None;
            window.focus(&self.controls[dropdown.trigger_focus_index()], cx);
        } else {
            self.open_dropdown = Some(dropdown);
            window.focus(&self.controls[8], cx);
        }
        cx.notify();
    }

    pub(super) fn render_dropdown(
        &self,
        id: &'static str,
        label: &'static str,
        dropdown: Dropdown,
        choices: Vec<(&'static str, bool, Choice)>,
        trigger_focus_index: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let open = self.open_dropdown == Some(dropdown);
        let trigger = div()
            .id(format!("{id}-trigger"))
            .role(Role::Button)
            .aria_label(label)
            .track_focus(&self.controls[trigger_focus_index])
            .tab_index(0)
            .tab_stop(true)
            .w(px(116.0))
            .h(px(30.0))
            .px(px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(if open {
                theme.control_selected
            } else {
                theme.raised
            })
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.text_secondary)
            .focus_visible(|style| style.border_color(theme.focus))
            .hover(|style| style.bg(theme.control_hover))
            .active(|style| style.bg(theme.control_active))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(label),
            )
            .child(icon(Icon::ChevronDown, 10.5, theme.text_tertiary))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, window, cx| {
                this.toggle_dropdown(dropdown, window, cx)
            });

        let menu = choices.into_iter().enumerate().fold(
            div()
                .id(format!("{id}-menu"))
                .absolute()
                .top(px(36.0))
                .right_0()
                .min_w(px(176.0))
                .p(px(4.0))
                .rounded(px(9.0))
                .border_1()
                .border_color(theme.border_strong)
                .bg(theme.raised)
                .shadow_lg()
                .occlude()
                .flex()
                .flex_col(),
            |menu, (index, (item_label, selected, choice))| {
                menu.child(
                    div()
                        .id(format!("{id}-option-{index}"))
                        .role(Role::Button)
                        .aria_label(item_label)
                        .track_focus(&self.controls[8 + index])
                        .tab_index(0)
                        .tab_stop(true)
                        .h(px(32.0))
                        .px(px(9.0))
                        .rounded(px(6.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.0))
                        .cursor_default()
                        .text_sm()
                        .text_color(if selected {
                            theme.text
                        } else {
                            theme.text_secondary
                        })
                        .hover(|style| style.bg(theme.control_hover))
                        .active(|style| style.bg(theme.control_active))
                        .focus_visible(|style| style.border_1().border_color(theme.focus))
                        .child(item_label)
                        .child(div().w(px(12.0)).when(selected, |mark| {
                            mark.child(icon(Icon::Check, 11.0, theme.text_tertiary))
                        }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_activation(cx, move |this, window, cx| {
                            this.apply_choice(choice, window, cx)
                        }),
                )
            },
        );

        div()
            .id(id)
            .relative()
            .flex()
            .justify_end()
            .child(trigger)
            .when(open, |control| {
                control.child(deferred(menu).with_priority(1))
            })
    }
}
