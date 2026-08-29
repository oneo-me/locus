use super::*;

impl Settings {
    fn render_setting_row(
        &self,
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
        cx: &App,
    ) -> gpui::Div {
        let theme = Theme::current(cx);
        div()
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
                            .text_color(theme.text_secondary)
                            .child(description),
                    ),
            )
            .child(control)
    }

    pub(super) fn render_general(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let path = self
            .preferences
            .library_path
            .as_deref()
            .and_then(Path::to_str)
            .unwrap_or(language.text("未选择", "Not selected"));
        let choose_label = language.text("选择…", "Choose…");
        let path_control = div()
            .min_w_0()
            .w(px(320.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .child(path.to_string()),
            )
            .child(
                div()
                    .id("choose-library")
                    .role(Role::Button)
                    .aria_label(choose_label)
                    .track_focus(&self.controls[3])
                    .tab_index(0)
                    .tab_stop(true)
                    .h(px(32.0))
                    .px(px(11.0))
                    .rounded(px(7.0))
                    .border_1()
                    .border_color(theme.border_strong)
                    .bg(theme.raised)
                    .flex()
                    .items_center()
                    .text_sm()
                    .text_color(theme.text_secondary)
                    .cursor_default()
                    .hover(|style| style.bg(theme.control_hover))
                    .active(|style| style.bg(theme.control_active))
                    .focus_visible(|style| style.border_color(theme.focus))
                    .child(choose_label)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_activation(cx, |this, _, cx| this.choose_library(cx)),
            );

        self.render_setting_row(
            language.text("文档库路径", "Library path"),
            language.text(
                "Markdown 笔记与附件所在的根目录",
                "Root folder containing Markdown notes and attachments",
            ),
            path_control,
            cx,
        )
    }

    pub(super) fn render_appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::current(cx);
        let language = self.preferences.language;
        let theme_choices = [
            (language.text("跟随系统", "System"), ThemePreference::System),
            (language.text("亮色", "Light"), ThemePreference::Light),
            (language.text("暗色", "Dark"), ThemePreference::Dark),
        ]
        .into_iter()
        .map(|(label, value)| (label, value == self.preferences.theme, Choice::Theme(value)))
        .collect();
        let language_choices = Language::ALL
            .into_iter()
            .map(|value| (value.label(), value == language, Choice::Language(value)))
            .collect();
        let ui_font_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.ui_font_size,
                    Choice::UiFont(value),
                )
            })
            .collect();
        let code_font_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.code_font_size,
                    Choice::CodeFont(value),
                )
            })
            .collect();
        let selected_theme = match self.preferences.theme {
            ThemePreference::System => language.text("跟随系统", "System"),
            ThemePreference::Light => language.text("亮色", "Light"),
            ThemePreference::Dark => language.text("暗色", "Dark"),
        };

        div()
            .flex()
            .flex_col()
            .child(self.render_setting_row(
                language.text("主题", "Theme"),
                language.text(
                    "控制窗口和内容表面的明暗外观",
                    "Controls window and content appearance",
                ),
                self.render_dropdown(
                    "theme-selector",
                    selected_theme,
                    Dropdown::Theme,
                    theme_choices,
                    4,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("语言", "Language"),
                language.text(
                    "切换界面与系统菜单使用的语言",
                    "Language used by the interface and application menus",
                ),
                self.render_dropdown(
                    "language-selector",
                    language.label(),
                    Dropdown::Language,
                    language_choices,
                    5,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("界面字号", "Interface font size"),
                language.text(
                    "调整整个窗口的界面文字大小",
                    "Adjusts interface text throughout the window",
                ),
                self.render_dropdown(
                    "ui-font-size-selector",
                    font_size_label(self.preferences.ui_font_size),
                    Dropdown::UiFont,
                    ui_font_choices,
                    6,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("代码字号", "Code font size"),
                language.text("用于代码内容的文字大小", "Font size used for code content"),
                self.render_dropdown(
                    "code-font-size-selector",
                    font_size_label(self.preferences.code_font_size),
                    Dropdown::CodeFont,
                    code_font_choices,
                    7,
                    cx,
                ),
                cx,
            ))
    }
}
