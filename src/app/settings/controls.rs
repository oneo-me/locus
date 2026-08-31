use super::*;

impl Settings {
    fn toggle_dropdown(&mut self, dropdown: Dropdown, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_dropdown == Some(dropdown) {
            self.open_dropdown = None;
            window.focus(&self.controls[dropdown.trigger_focus_index()], cx);
        } else {
            self.open_dropdown = Some(dropdown);
            if matches!(
                dropdown,
                Dropdown::UiFontFamily | Dropdown::CodeFontFamily | Dropdown::TerminalFontFamily
            ) {
                self.font_search.clear();
                self.font_search_input
                    .update(cx, |input, cx| input.set_value("", cx));
                window.focus(&self.font_search_input.focus_handle(cx), cx);
            } else {
                window.focus(&self.controls[FOCUS_MENU_START], cx);
            }
        }
        cx.notify();
    }

    fn choose_font(
        &mut self,
        kind: FontKind,
        family: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_dropdown = None;
        window.focus(&self.controls[kind.focus_index()], cx);
        if kind.selected(&self.preferences) == family {
            cx.notify();
            return;
        }
        match kind {
            FontKind::Ui => {
                self.preferences.ui_font_family = family;
                cx.emit(SettingsEvent::UiFontChanged);
            }
            FontKind::Code => {
                self.preferences.code_font_family = family;
                cx.emit(SettingsEvent::CodeFontChanged);
            }
            FontKind::Terminal => {
                self.preferences.terminal_font_family = family.clone();
                cx.emit(SettingsEvent::TerminalFontChanged(
                    family,
                    self.preferences.terminal_font_size,
                ));
            }
        }
        self.persist();
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
        let theme = Theme::global(cx).clone();
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
            .border_color(theme.input)
            .bg(if open { theme.accent } else { theme.secondary })
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.muted_foreground)
            .focus_visible(|style| style.border_color(theme.ring))
            .hover(|style| style.bg(theme.accent))
            .active(|style| style.bg(theme.accent))
            .child(
                div()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .child(label),
            )
            .child(icon(Icon::ChevronDown, 10.5, theme.muted_foreground))
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
                .border_color(theme.input)
                .bg(theme.secondary)
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
                        .track_focus(&self.controls[FOCUS_MENU_START + index])
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
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .hover(|style| style.bg(theme.accent))
                        .active(|style| style.bg(theme.accent))
                        .focus_visible(|style| style.border_1().border_color(theme.ring))
                        .child(item_label)
                        .child(div().w(px(12.0)).when(selected, |mark| {
                            mark.child(icon(Icon::Check, 11.0, theme.muted_foreground))
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

    pub(super) fn render_font_picker(
        &self,
        kind: FontKind,
        cx: &mut Context<Self>,
    ) -> Stateful<gpui::Div> {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let dropdown = kind.dropdown();
        let id = kind.id();
        let open = self.open_dropdown == Some(dropdown);
        let selected = kind.selected(&self.preferences).to_owned();
        let query = self.font_search.trim().to_lowercase();
        let matches = self
            .font_names
            .iter()
            .enumerate()
            .filter(|(_, family)| font_matches_search(family, &query))
            .collect::<Vec<_>>();
        let has_matches = !matches.is_empty();
        let trigger = div()
            .id(format!("{id}-trigger"))
            .role(Role::ComboBox)
            .aria_label(kind.label(language))
            .aria_value(selected.clone())
            .track_focus(&self.controls[kind.focus_index()])
            .tab_index(0)
            .tab_stop(true)
            .w_full()
            .h(px(30.0))
            .px(px(10.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.input)
            .bg(if open { theme.accent } else { theme.secondary })
            .flex()
            .items_center()
            .justify_between()
            .gap(px(6.0))
            .cursor_default()
            .text_sm()
            .text_color(theme.muted_foreground)
            .focus_visible(|style| style.border_color(theme.ring))
            .hover(|style| style.bg(theme.accent))
            .active(|style| style.bg(theme.accent))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .font_family(selected.clone())
                    .child(selected),
            )
            .child(icon(Icon::ChevronDown, 10.5, theme.muted_foreground))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_activation(cx, move |this, window, cx| {
                this.toggle_dropdown(dropdown, window, cx)
            });

        let options = matches.into_iter().fold(
            div()
                .id(format!("{id}-options"))
                .role(Role::List)
                .max_h(px(300.0))
                .overflow_y_scroll()
                .p(px(4.0))
                .flex()
                .flex_col(),
            |options, (index, family)| {
                let family = family.clone();
                let is_selected = family == kind.selected(&self.preferences);
                options.child(
                    div()
                        .id(format!("{id}-option-{index}"))
                        .role(Role::ListBoxOption)
                        .aria_label(family.clone())
                        .aria_selected(is_selected)
                        .track_focus(&self.font_option_focus[index])
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
                            theme.foreground
                        } else {
                            theme.muted_foreground
                        })
                        .hover(|style| style.bg(theme.accent))
                        .active(|style| style.bg(theme.accent))
                        .focus_visible(|style| style.border_1().border_color(theme.ring))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .font_family(family.clone())
                                .child(family.clone()),
                        )
                        .child(div().w(px(12.0)).when(is_selected, |mark| {
                            mark.child(icon(Icon::Check, 11.0, theme.muted_foreground))
                        }))
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .on_activation(cx, move |this, window, cx| {
                            this.choose_font(kind, family.clone(), window, cx)
                        }),
                )
            },
        );
        let menu = div()
            .id(format!("{id}-menu"))
            .absolute()
            .top(px(36.0))
            .right_0()
            .w(px(328.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(theme.input)
            .bg(theme.secondary)
            .shadow_lg()
            .occlude()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(div().p(px(4.0)).child(self.font_search_input.clone()))
            .child(div().mx(px(4.0)).h(px(1.0)).bg(theme.border))
            .when(self.font_names.is_empty(), |menu| {
                menu.child(
                    div()
                        .px(px(12.0))
                        .py(px(10.0))
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(language.text("没有可用字体", "No fonts available")),
                )
            })
            .when(
                !self.font_names.is_empty() && !query.is_empty() && !has_matches,
                |menu| {
                    menu.child(
                        div()
                            .px(px(12.0))
                            .py(px(10.0))
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(language.text("没有匹配字体", "No matching fonts")),
                    )
                },
            )
            .child(options);

        div()
            .id(id)
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
