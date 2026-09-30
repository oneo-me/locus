use super::*;
use gpui_kit::component::{
    Sizable as _,
    button::{Button, ButtonVariants as _},
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
        let path_control = directory_control(
            path,
            Button::new("choose-library")
                .label(choose_label)
                .on_click(cx.listener(|this, _, _, cx| this.choose_library(cx))),
            &theme,
        );
        let export_path = self
            .preferences
            .resolved_export_directory()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| {
                t!("settings.export.not_selected", locale = language.locale()).into_owned()
            });
        let export_control = directory_control(
            export_path.clone(),
            Button::new("choose-export-directory")
                .accessibility_label(
                    t!("settings.export.choose_prompt", locale = language.locale()).into_owned(),
                )
                .tooltip(export_path)
                .label(t!("settings.export.choose", locale = language.locale()).into_owned())
                .on_click(cx.listener(|this, _, _, cx| this.choose_export_directory(cx))),
            &theme,
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
                t!("settings.export.directory", locale = language.locale()).into_owned(),
                t!("settings.export.description", locale = language.locale()).into_owned(),
                export_control,
            ))
            .child(div().mx(ui_rem(20.0)).h(px(1.0)).bg(theme.border))
            .child(SettingsItem::new(
                t!("settings.editor.label", locale = language.locale()).into_owned(),
                t!("settings.editor.description", locale = language.locale()).into_owned(),
                self.render_dropdown(Dropdown::Editor, cx),
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
        let language = self.preferences.appearance.language;
        div()
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .child(SettingsItem::new(
                t!(
                    "settings.terminal.application.label",
                    locale = language.locale()
                )
                .into_owned(),
                t!(
                    "settings.terminal.application.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::Terminal, cx),
            ))
            .child(div().mx_5().h(px(1.0)).bg(Theme::global(cx).border))
            .child(SettingsItem::new(
                t!("settings.terminal.agent.label", locale = language.locale()).into_owned(),
                t!(
                    "settings.terminal.agent.description",
                    locale = language.locale()
                )
                .into_owned(),
                self.render_dropdown(Dropdown::Agent, cx),
            ))
    }
}

fn directory_control(path: String, button: Button, theme: &Theme) -> gpui_kit::Div {
    div()
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
        .child(button.text().small().h_full().px(ui_rem(10.0)))
}
