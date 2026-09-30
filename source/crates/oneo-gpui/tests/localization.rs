#![cfg(feature = "shell")]
use oneo_gpui::Language;

#[test]
fn language_changes_update_application_and_component_locale() {
    for language in [
        Language::System,
        Language::English,
        Language::Chinese,
        Language::English,
    ] {
        language.activate();
        assert_eq!(&*rust_i18n::locale(), language.locale());
        assert_eq!(&*gpui_kit::component::locale(), language.locale());
    }
}
