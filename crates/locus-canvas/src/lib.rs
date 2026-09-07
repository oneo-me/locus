rust_i18n::i18n!("locales", fallback = "en");

mod view;

pub use view::{CanvasEvent, CanvasView};

#[cfg(test)]
mod tests {
    #[test]
    fn catalog_resolves_both_languages() {
        let source = include_str!("../locales/ui.yml")
            .split_once('\n')
            .unwrap()
            .1;
        let messages: std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<String, String>,
        > = serde_saphyr::from_str(source).unwrap();
        for (key, translations) in messages {
            for locale in ["en", "zh-CN"] {
                let expected = &translations[locale];
                assert!(!expected.is_empty());
                assert_eq!(rust_i18n::t!(&key, locale = locale).as_ref(), expected);
            }
        }
    }
}
