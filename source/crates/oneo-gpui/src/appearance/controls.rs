use super::{
    AppearanceChange, AppearancePreferences, FONT_SIZES, Language, ThemePreference, label,
};
use gpui_kit::component::{
    ActiveTheme, IndexPath,
    select::{Select, SelectEvent, SelectItem, SelectState},
};
use gpui_kit::{
    App, Context, Div, Entity, SharedString, Subscription, Window, div, prelude::*, px, rems,
};

#[derive(Clone)]
struct Choice {
    label: SharedString,
    value: AppearanceChange,
}

impl SelectItem for Choice {
    type Value = AppearanceChange;
    fn title(&self) -> SharedString {
        self.label.clone()
    }
    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Clone, Copy)]
enum Field {
    Theme,
    Language,
    FontSize,
}

impl Field {
    fn key(self) -> &'static str {
        match self {
            Self::Theme => "settings.appearance.theme",
            Self::Language => "settings.appearance.language",
            Self::FontSize => "settings.appearance.interface_font_size",
        }
    }

    fn selected(self, preferences: AppearancePreferences) -> AppearanceChange {
        match self {
            Self::Theme => AppearanceChange::Theme(preferences.theme),
            Self::Language => AppearanceChange::Language(preferences.language),
            Self::FontSize => AppearanceChange::FontSize(preferences.ui_font_size),
        }
    }

    fn choices(self, language: Language) -> Vec<Choice> {
        let values = match self {
            Self::Theme => [
                ThemePreference::System,
                ThemePreference::Light,
                ThemePreference::Dark,
            ]
            .into_iter()
            .map(AppearanceChange::Theme)
            .collect(),
            Self::Language => Language::ALL
                .into_iter()
                .map(AppearanceChange::Language)
                .collect(),
            Self::FontSize => FONT_SIZES
                .into_iter()
                .map(AppearanceChange::FontSize)
                .collect::<Vec<_>>(),
        };
        values
            .into_iter()
            .map(|value| {
                let text = match value {
                    AppearanceChange::Theme(theme) => label(
                        match theme {
                            ThemePreference::System => "settings.appearance.theme.system",
                            ThemePreference::Light => "settings.appearance.theme.light",
                            ThemePreference::Dark => "settings.appearance.theme.dark",
                        },
                        language,
                    ),
                    AppearanceChange::Language(value) => value.label(language),
                    AppearanceChange::FontSize(size) => size.to_string(),
                };
                Choice {
                    label: text.into(),
                    value,
                }
            })
            .collect()
    }
}

/// Retains selector state; the parent owns preferences, persistence, and change handling.
pub struct AppearanceControls {
    fields: Vec<(Field, Entity<SelectState<Vec<Choice>>>)>,
    _subscriptions: Vec<Subscription>,
}

impl AppearanceControls {
    pub fn new<T: 'static>(
        preferences: AppearancePreferences,
        window: &mut Window,
        cx: &mut Context<T>,
        on_change: fn(&mut T, AppearanceChange, &mut Window, &mut Context<T>),
    ) -> Self {
        let mut fields = Vec::new();
        let mut subscriptions = Vec::new();
        for field in [Field::Theme, Field::Language, Field::FontSize] {
            let items = field.choices(preferences.language);
            let selected = items
                .iter()
                .position(|item| item.value == field.selected(preferences))
                .map(|ix| IndexPath::default().row(ix));
            let select = cx.new(|cx| SelectState::new(items, selected, window, cx));
            subscriptions.push(cx.subscribe_in(
                &select,
                window,
                move |this, _, event: &SelectEvent<Vec<Choice>>, window, cx| {
                    if let SelectEvent::Confirm(Some(value)) = event {
                        on_change(this, *value, window, cx);
                    }
                },
            ));
            fields.push((field, select));
        }
        Self {
            fields,
            _subscriptions: subscriptions,
        }
    }

    pub fn sync(&self, preferences: AppearancePreferences, window: &mut Window, cx: &mut App) {
        for (field, select) in &self.fields {
            select.update(cx, |select, cx| {
                select.set_items(field.choices(preferences.language), window, cx);
                select.set_selected_value(&field.selected(preferences), window, cx);
            });
        }
    }

    pub fn render(&self, language: Language, cx: &App) -> Div {
        let theme = cx.theme();
        let mut content = div().w_full().min_w_0().flex().flex_col().items_stretch();
        for (ix, (field, select)) in self.fields.iter().enumerate() {
            let field = *field;
            if ix > 0 {
                content = content.child(div().mx_5().h(px(1.0)).bg(theme.border));
            }
            let title = label(&format!("{}.label", field.key()), language);
            content = content.child(crate::SettingsItem::new(
                title.clone(),
                label(&format!("{}.description", field.key()), language),
                div()
                    .debug_selector(move || field.key().to_owned())
                    .w(rems(9.0))
                    .flex_none()
                    .child(Select::new(select).w_full().accessibility_label(title)),
            ));
        }
        content
    }
}
