use gpui_component::select::Select;
use rust_i18n::t;

use super::*;

impl Settings {
    pub(super) fn choose_font(
        &mut self,
        kind: FontKind,
        family: String,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if kind.selected(&self.preferences) == family {
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
        dropdown: Dropdown,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let language = self.preferences.language;
        div().w(gpui::rems(7.25)).flex_none().child(
            Select::new(&self.choice_selects[&dropdown])
                .w_full()
                .accessibility_label(dropdown.label(language)),
        )
    }

    pub(super) fn render_font_picker(
        &self,
        kind: FontKind,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let language = self.preferences.language;
        div().w(gpui::rems(20.5)).flex_none().child(
            Select::new(&self.font_selects[&kind])
                .w_full()
                .menu_width(gpui::rems(20.5))
                .search_placeholder(
                    t!("settings.font.search", locale = language.locale()).into_owned(),
                )
                .accessibility_label(kind.label(language))
                .empty(move |_, cx| {
                    div()
                        .h_16()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_sm()
                        .text_color(Theme::global(cx).muted_foreground)
                        .child(
                            t!("settings.font.no_matches", locale = language.locale()).into_owned(),
                        )
                }),
        )
    }
}
