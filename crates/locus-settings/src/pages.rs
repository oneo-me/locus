use super::*;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
    switch::Switch,
};
use rust_i18n::t;

impl Settings {
    pub(super) fn render_general(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.appearance.language;
        let path = self
            .preferences
            .library_path
            .as_deref()
            .and_then(Path::to_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                t!("settings.library.not_selected", locale = language.locale()).into_owned()
            });
        let choose_label = t!("settings.library.choose", locale = language.locale()).into_owned();
        let path_control = div()
            .min_w_0()
            .flex_none()
            .w(gpui_kit::rems(SETTINGS_CONTROL_WIDTH_REM))
            .h(ui_rem(32.0))
            .rounded(ui_rem(7.0))
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
                    .px(ui_rem(10.0))
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
                    .px(ui_rem(10.0))
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
            .child(SettingsItem::new(
                t!("settings.library.path.label", locale = language.locale()).into_owned(),
                t!(
                    "settings.library.path.description",
                    locale = language.locale()
                )
                .into_owned(),
                path_control,
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.editor.label", locale = language.locale()).into_owned(),
                t!("settings.editor.description", locale = language.locale()).into_owned(),
                self.render_dropdown(Dropdown::Editor, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.auto_open_canvas.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.auto_open_canvas.description",
                    locale = language.locale()
                )
                .into_owned(),
                Switch::new("auto-open-canvas")
                    .small()
                    .checked(self.preferences.auto_open_canvas)
                    .accessibility_label(
                        t!(
                            "settings.auto_open_canvas.label",
                            locale = language.locale()
                        )
                        .into_owned(),
                    )
                    .on_click(cx.listener(|this, checked, _, cx| {
                        this.preferences.auto_open_canvas = *checked;
                        this.persist();
                        cx.notify();
                    })),
            ))
    }

    pub(super) fn render_appearance(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.appearance.language;
        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(self.appearance_controls.render(language, cx))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.appearance.interface_font.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.appearance.interface_font.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_font_picker(FontKind::Ui, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.appearance.code_font.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.appearance.code_font.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_font_picker(FontKind::Code, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.appearance.code_font_size.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.appearance.code_font_size.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::CodeFontSize, cx),
            ))
    }

    pub(super) fn render_terminal(&self, cx: &mut Context<Self>) -> gpui_kit::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.appearance.language;
        div()
            .w_full()
            .self_stretch()
            .min_w_0()
            .flex()
            .flex_col()
            .items_stretch()
            .child(SettingsItem::new(
                t!("settings.terminal.font.label", locale = language.locale()).into_owned(),
                t!(
                    "settings.terminal.font.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_font_picker(FontKind::Terminal, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.terminal.font_size.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.terminal.font_size.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::TerminalFontSize, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.terminal.shell.label", locale = language.locale()).into_owned(),
                t!(
                    "settings.terminal.shell.description",
                    locale = language.locale()
                )
                .into_owned(),
                Input::new(&self.terminal_shell_input)
                    .flex_none()
                    .w(gpui_kit::rems(SETTINGS_CONTROL_WIDTH_REM))
                    .aria_label(
                        t!("settings.terminal.shell.label", locale = language.locale())
                            .into_owned(),
                    ),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.terminal.command.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.terminal.command.description",
                    locale = language.locale()
                )
                .into_owned(),
                Input::new(&self.terminal_command_input)
                    .flex_none()
                    .w(gpui_kit::rems(SETTINGS_CONTROL_WIDTH_REM))
                    .aria_label(
                        t!(
                            "settings.terminal.command.label",
                            locale = language.locale()
                        )
                        .into_owned(),
                    ),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.terminal.color_scheme.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.terminal.color_scheme.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::TerminalColorScheme, cx),
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!(
                    "settings.terminal.cursor_style.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.terminal.cursor_style.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::TerminalCursorStyle, cx),
            ))
    }
}
