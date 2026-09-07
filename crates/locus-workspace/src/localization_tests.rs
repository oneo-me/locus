use rust_i18n::t;
use serde_json::Value;

#[test]
fn package_catalog_resolves_every_message_in_both_languages() {
    let messages: serde_json::Map<String, Value> =
        serde_saphyr::from_str(include_str!("../locales/ui.yml")).unwrap();
    for (key, translations) in messages {
        if key == "_version" {
            continue;
        }
        for locale in ["en", "zh-CN"] {
            let expected = translations[locale]
                .as_str()
                .expect("translation is present");
            assert!(!expected.is_empty(), "{key} is empty in {locale}");
            assert_eq!(t!(&key, locale = locale), expected, "{key} in {locale}");
        }
    }
}

#[test]
fn empty_sidebar_follows_language_switches() {
    use oneo_gpui::Language;

    for (language, expected) in [
        (Language::Chinese, "暂无内容"),
        (Language::English, "No content"),
        (Language::Chinese, "暂无内容"),
    ] {
        language.activate();
        assert_eq!(t!("workspace.sidebar.empty"), expected);
    }
}

#[test]
fn terminal_labels_substitute_the_session_id() {
    assert_eq!(
        t!("workspace.terminal.tab", id = 3, locale = "en"),
        "Terminal 3"
    );
    assert_eq!(
        t!("workspace.terminal.close_tab", id = 3, locale = "zh-CN"),
        "关闭终端 3"
    );
}
