use super::*;
use gpui_component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
};

impl Settings {
    pub(super) fn render_setting_row(
        &self,
        title: impl Into<gpui::SharedString>,
        description: impl Into<gpui::SharedString>,
        control: impl IntoElement,
        cx: &App,
    ) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let title = title.into();
        let description = description.into();
        div()
            .w_full()
            .self_stretch()
            .min_h(px(68.0))
            .px(px(20.0))
            .py(px(13.0))
            .flex()
            .items_center()
            .gap(px(24.0))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .mt(px(4.0))
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(description),
                    ),
            )
            .child(control)
    }

    pub(super) fn render_general(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let path = self
            .preferences
            .library_path
            .as_deref()
            .and_then(Path::to_str)
            .map(str::to_owned)
            .unwrap_or_else(|| language.text("settings.library.not_selected"));
        let choose_label = language.text("settings.library.choose");
        let path_control = div()
            .min_w_0()
            .flex_none()
            .w(gpui::rems(SETTINGS_CONTROL_WIDTH_REM))
            .h(px(32.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(theme.input)
            .bg(theme.background)
            .overflow_hidden()
            .flex()
            .items_center()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .px(px(10.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(path),
            )
            .child(
                Button::new("choose-library")
                    .text()
                    .small()
                    .h_full()
                    .px(px(10.0))
                    .label(choose_label)
                    .on_click(cx.listener(|this, _, _, cx| this.choose_library(cx))),
            );

        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(self.render_setting_row(
                language.text("settings.library.path.label"),
                language.text("settings.library.path.description"),
                path_control,
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.editor.label"),
                language.text("settings.editor.description"),
                self.render_dropdown(Dropdown::Editor, cx),
                cx,
            ))
    }

    pub(super) fn render_appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(self.render_setting_row(
                language.text("settings.appearance.theme.label"),
                language.text("settings.appearance.theme.description"),
                self.render_dropdown(Dropdown::Theme, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.appearance.language.label"),
                language.text("settings.appearance.language.description"),
                self.render_dropdown(Dropdown::Language, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.appearance.interface_font.label"),
                language.text("settings.appearance.interface_font.description"),
                self.render_font_picker(FontKind::Ui, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.appearance.interface_font_size.label"),
                language.text("settings.appearance.interface_font_size.description"),
                self.render_dropdown(Dropdown::UiFontSize, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.appearance.code_font.label"),
                language.text("settings.appearance.code_font.description"),
                self.render_font_picker(FontKind::Code, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.appearance.code_font_size.label"),
                language.text("settings.appearance.code_font_size.description"),
                self.render_dropdown(Dropdown::CodeFontSize, cx),
                cx,
            ))
    }

    pub(super) fn render_terminal(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(self.render_setting_row(
                language.text("settings.terminal.font.label"),
                language.text("settings.terminal.font.description"),
                self.render_font_picker(FontKind::Terminal, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.terminal.font_size.label"),
                language.text("settings.terminal.font_size.description"),
                self.render_dropdown(Dropdown::TerminalFontSize, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(
                self.render_setting_row(
                    language.text("settings.terminal.shell.label"),
                    language.text("settings.terminal.shell.description"),
                    Input::new(&self.terminal_shell_input)
                        .flex_none()
                        .w(gpui::rems(SETTINGS_CONTROL_WIDTH_REM))
                        .aria_label(language.text("settings.terminal.shell.label")),
                    cx,
                ),
            )
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(
                self.render_setting_row(
                    language.text("settings.terminal.command.label"),
                    language.text("settings.terminal.command.description"),
                    Input::new(&self.terminal_command_input)
                        .flex_none()
                        .w(gpui::rems(SETTINGS_CONTROL_WIDTH_REM))
                        .aria_label(language.text("settings.terminal.command.label")),
                    cx,
                ),
            )
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.terminal.color_scheme.label"),
                language.text("settings.terminal.color_scheme.description"),
                self.render_dropdown(Dropdown::TerminalColorScheme, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("settings.terminal.cursor_style.label"),
                language.text("settings.terminal.cursor_style.description"),
                self.render_dropdown(Dropdown::TerminalCursorStyle, cx),
                cx,
            ))
    }
}
