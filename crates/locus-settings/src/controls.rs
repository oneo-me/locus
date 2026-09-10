use gpui_kit::component::select::Select;
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
        }
        self.persist();
        cx.notify();
    }

    pub(super) fn render_dropdown(
        &self,
        dropdown: Dropdown,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let language = self.preferences.appearance.language;
        let width = match dropdown {
            Dropdown::Terminal | Dropdown::Agent => 12.0,
            _ => 7.25,
        };
        div().w(gpui_kit::rems(width)).flex_none().child(
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
        let language = self.preferences.appearance.language;
        div().w(gpui_kit::rems(20.5)).flex_none().child(
            Select::new(&self.font_selects[&kind])
                .w_full()
                .menu_width(gpui_kit::rems(20.5))
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
