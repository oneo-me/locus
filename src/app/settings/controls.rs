use super::*;

const MENU_FOCUS_START: usize = 16;

impl Settings {
    fn toggle_dropdown(&mut self, dropdown: Dropdown, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_dropdown == Some(dropdown) {
            self.open_dropdown = None;
            window.focus(&self.controls[dropdown.trigger_focus_index()], cx);
        } else {
            self.open_dropdown = Some(dropdown);
            if dropdown == Dropdown::CodeFontFamily {
                self.code_font_search.clear();
                self.code_font_search_input
                    .update(cx, |input, cx| input.set_value("", cx));
                window.focus(&self.code_font_search_input.focus_handle(cx), cx);
            } else {
                window.focus(&self.controls[MENU_FOCUS_START], cx);
            }
        }
        cx.notify();
    }

    fn choose_code_font(&mut self, family: String, window: &mut Window, cx: &mut Context<Self>) {
        self.open_dropdown = None;
        window.focus(&self.controls[FOCUS_CODE_FONT_FAMILY], cx);
        if self.preferences.code_font_family == family {
            cx.notify();
            return;
        }
        self.preferences.code_font_family = family.clone();
        self.persist();
        cx.emit(SettingsEvent::CodeFontChanged(
            family,
            self.preferences.code_font_size,
        ));
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
                        .track_focus(&self.controls[MENU_FOCUS_START + index])
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

    pub(super) fn render_code_font_picker(&self, cx: &mut Context<Self>) -> Stateful<gpui::Div> {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let open = self.open_dropdown == Some(Dropdown::CodeFontFamily);
        let selected = self.preferences.resolved_code_font_family().to_owned();
        let query = self.code_font_search.trim().to_lowercase();
        let matches = self
            .code_font_names
            .iter()
            .enumerate()
            .filter(|(_, family)| font_matches_search(family, &query))
            .collect::<Vec<_>>();
        let has_matches = !matches.is_empty();
        let trigger = div()
            .id("code-font-family-selector-trigger")
            .role(Role::ComboBox)
            .aria_label(language.text("代码字体", "Code font"))
            .aria_value(selected.clone())
            .track_focus(&self.controls[FOCUS_CODE_FONT_FAMILY])
            .tab_index(0)
            .tab_stop(true)
            .w_full()
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
                    .flex_1()
                    .truncate()
                    .font_family(selected.clone())
                    .child(selected),
            )
            .child(icon(Icon::ChevronDown, 10.5, theme.text_tertiary))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, |this, window, cx| {
                this.toggle_dropdown(Dropdown::CodeFontFamily, window, cx)
            });

        let options = matches.into_iter().fold(
            div()
                .id("code-font-options")
                .role(Role::List)
                .max_h(px(300.0))
                .overflow_y_scroll()
                .p(px(4.0))
                .flex()
                .flex_col(),
            |options, (index, family)| {
                let family = family.clone();
                let is_selected = family == self.preferences.resolved_code_font_family();
                options.child(
                    div()
                        .id(format!("code-font-option-{index}"))
                        .role(Role::ListBoxOption)
                        .aria_label(family.clone())
                        .aria_selected(is_selected)
                        .track_focus(&self.code_font_option_focus[index])
                        .tab_index(0)
                        .tab_stop(true)
                        .h(px(34.0))
                        .px(px(9.0))
                        .rounded(px(6.0))
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(10.0))
                        .cursor_default()
                        .text_sm()
                        .text_color(if is_selected {
                            theme.text
                        } else {
                            theme.text_secondary
                        })
                        .hover(|style| style.bg(theme.control_hover))
                        .active(|style| style.bg(theme.control_active))
                        .focus_visible(|style| style.border_1().border_color(theme.focus))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .font_family(family.clone())
                                .child(family.clone()),
                        )
                        .child(div().w(px(12.0)).when(is_selected, |mark| {
                            mark.child(icon(Icon::Check, 11.0, theme.text_tertiary))
                        }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_activation(cx, move |this, window, cx| {
                            this.choose_code_font(family.clone(), window, cx)
                        }),
                )
            },
        );
        let menu = div()
            .id("code-font-family-selector-menu")
            .absolute()
            .top(px(36.0))
            .right_0()
            .w(px(328.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(theme.border_strong)
            .bg(theme.raised)
            .shadow_lg()
            .occlude()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().p(px(4.0)).child(self.code_font_search_input.clone()))
            .child(div().mx(px(4.0)).h(px(1.0)).bg(theme.border))
            .when(self.code_font_names.is_empty(), |menu| {
                menu.child(
                    div()
                        .px(px(12.0))
                        .py(px(10.0))
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child(language.text("没有可用字体", "No fonts available")),
                )
            })
            .when(
                !self.code_font_names.is_empty() && !query.is_empty() && !has_matches,
                |menu| {
                    menu.child(
                        div()
                            .px(px(12.0))
                            .py(px(10.0))
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(language.text("没有匹配字体", "No matching fonts")),
                    )
                },
            )
            .child(options);

        div()
            .id("code-font-family-selector")
            .relative()
            .w(px(328.0))
            .flex()
            .justify_end()
            .child(trigger)
            .when(open, |control| {
                control.child(deferred(menu).with_priority(2))
            })
    }
}
