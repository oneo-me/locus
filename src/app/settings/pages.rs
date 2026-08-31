use super::*;

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
            .unwrap_or(language.text("未选择", "Not selected"));
        let choose_label = language.text("选择…", "Choose…");
        let editor_choices = EditorPreference::ALL
            .into_iter()
            .map(|value| {
                (
                    value.label(language),
                    value == self.preferences.editor,
                    Choice::Editor(value),
                )
            })
            .collect();
        let path_control = div()
            .min_w_0()
            .w(px(320.0))
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
                    .child(path.to_string()),
            )
            .child(
                div()
                    .id("choose-library")
                    .role(Role::Button)
                    .aria_label(choose_label)
                    .track_focus(&self.controls[FOCUS_LIBRARY_PICKER])
                    .tab_index(0)
                    .tab_stop(true)
                    .h_full()
                    .px(px(11.0))
                    .rounded_r(px(6.0))
                    .border_l_1()
                    .border_color(theme.input)
                    .flex()
                    .items_center()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .cursor_default()
                    .hover(|style| style.bg(theme.accent))
                    .active(|style| style.bg(theme.accent))
                    .focus_visible(|style| style.border_1().border_color(theme.ring))
                    .child(choose_label)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_activation(cx, |this, _, cx| this.choose_library(cx)),
            );

        div()
            .flex()
            .flex_col()
            .child(self.render_setting_row(
                language.text("文档库路径", "Library path"),
                language.text(
                    "Markdown 笔记与附件所在的根目录",
                    "Root folder containing Markdown notes and attachments",
                ),
                path_control,
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("编辑器", "Editor"),
                language.text(
                    "笔记页面的编辑按钮使用此外部编辑器",
                    "External editor used by the note page edit button",
                ),
                self.render_dropdown(
                    "editor-selector",
                    self.preferences.editor.label(language),
                    Dropdown::Editor,
                    editor_choices,
                    FOCUS_EDITOR,
                    cx,
                ),
                cx,
            ))
    }

    pub(super) fn render_appearance(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
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
                    Choice::UiFontSize(value),
                )
            })
            .collect();
        let code_font_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.code_font_size,
                    Choice::CodeFontSize(value),
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
                    FOCUS_THEME,
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
                    FOCUS_LANGUAGE,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("软件字体", "Interface font"),
                language.text(
                    "用于界面与笔记正文的字体",
                    "Font used by the interface and note text",
                ),
                self.render_font_picker(FontKind::Ui, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("软件字号", "Interface font size"),
                language.text(
                    "调整界面与笔记正文的文字大小",
                    "Text size used by the interface and note text",
                ),
                self.render_dropdown(
                    "ui-font-size-selector",
                    font_size_label(self.preferences.ui_font_size),
                    Dropdown::UiFontSize,
                    ui_font_choices,
                    FOCUS_UI_FONT_SIZE,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("代码字体", "Code font"),
                language.text(
                    "用于笔记和设置中的代码内容",
                    "Font used for code in notes and settings",
                ),
                self.render_font_picker(FontKind::Code, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("代码字号", "Code font size"),
                language.text(
                    "用于笔记和设置中的代码内容",
                    "Text size used for code in notes and settings",
                ),
                self.render_dropdown(
                    "code-font-size-selector",
                    font_size_label(self.preferences.code_font_size),
                    Dropdown::CodeFontSize,
                    code_font_choices,
                    FOCUS_CODE_FONT_SIZE,
                    cx,
                ),
                cx,
            ))
    }

    pub(super) fn render_terminal(&self, cx: &mut Context<Self>) -> gpui::Div {
        let theme = Theme::global(cx).clone();
        let language = self.preferences.language;
        let font_size_choices = FONT_SIZES
            .into_iter()
            .map(|value| {
                (
                    font_size_label(value),
                    value == self.preferences.terminal_font_size,
                    Choice::TerminalFontSize(value),
                )
            })
            .collect();
        let color_choices = TerminalColorScheme::ALL
            .into_iter()
            .map(|value| {
                (
                    terminal_color_scheme_label(value, language),
                    value == self.preferences.terminal_color_scheme,
                    Choice::TerminalColorScheme(value),
                )
            })
            .collect();
        let cursor_choices = TerminalCursorStyle::ALL
            .into_iter()
            .map(|value| {
                (
                    terminal_cursor_style_label(value, language),
                    value == self.preferences.terminal_cursor_style,
                    Choice::TerminalCursorStyle(value),
                )
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .child(self.render_setting_row(
                language.text("终端字体", "Terminal font"),
                language.text(
                    "用于右侧终端会话的字体",
                    "Font used by the terminal in the right sidebar",
                ),
                self.render_font_picker(FontKind::Terminal, cx),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("终端字号", "Terminal font size"),
                language.text(
                    "用于右侧终端会话的文字大小",
                    "Text size used by the terminal in the right sidebar",
                ),
                self.render_dropdown(
                    "terminal-font-size-selector",
                    font_size_label(self.preferences.terminal_font_size),
                    Dropdown::TerminalFontSize,
                    font_size_choices,
                    FOCUS_TERMINAL_FONT_SIZE,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("启动 Shell", "Launch shell"),
                language.text(
                    "留空时使用系统登录 Shell",
                    "Uses the system login shell when empty",
                ),
                self.terminal_shell_input.clone(),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("默认命令", "Default command"),
                language.text(
                    "每次启动终端后执行，留空则直接进入 Shell",
                    "Runs after each terminal starts; leave empty for the shell prompt",
                ),
                self.terminal_command_input.clone(),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("配色", "Color scheme"),
                language.text(
                    "自动跟随 Locus 的当前明暗外观",
                    "Auto follows the current Locus appearance",
                ),
                self.render_dropdown(
                    "terminal-color-scheme-selector",
                    terminal_color_scheme_label(self.preferences.terminal_color_scheme, language),
                    Dropdown::TerminalColorScheme,
                    color_choices,
                    FOCUS_TERMINAL_COLOR_SCHEME,
                    cx,
                ),
                cx,
            ))
            .child(div().mx(px(20.0)).h(px(1.0)).bg(theme.border))
            .child(self.render_setting_row(
                language.text("光标样式", "Cursor style"),
                language.text(
                    "作为终端程序未指定光标时的默认样式",
                    "Default used when a terminal program does not set the cursor",
                ),
                self.render_dropdown(
                    "terminal-cursor-style-selector",
                    terminal_cursor_style_label(self.preferences.terminal_cursor_style, language),
                    Dropdown::TerminalCursorStyle,
                    cursor_choices,
                    FOCUS_TERMINAL_CURSOR_STYLE,
                    cx,
                ),
                cx,
            ))
    }
}
