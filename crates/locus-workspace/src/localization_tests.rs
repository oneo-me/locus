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
