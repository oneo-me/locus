use rust_i18n::t;
use serde::{Deserialize, Serialize};

/// Registers component translations once, before initializing GPUI Component.
pub fn init() {
    rust_i18n::extend!(gpui_component);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Chinese, Self::English];

    pub fn label(self) -> String {
        match self {
            Self::Chinese => self.text("language.chinese"),
            Self::English => self.text("language.english"),
        }
    }

    pub fn text(self, key: &str) -> String {
        t!(key, locale = self.locale()).into_owned()
    }

    pub fn text_with(self, key: &str, arguments: &[(&str, String)]) -> String {
        let patterns: Vec<_> = arguments.iter().map(|(name, _)| *name).collect();
        let values: Vec<_> = arguments.iter().map(|(_, value)| value.clone()).collect();
        rust_i18n::replace_patterns(&self.text(key), &patterns, &values)
    }

    pub fn locale(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }

    pub fn activate(self) {
        rust_i18n::set_locale(self.locale());
        gpui_component::set_locale(self.locale());
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn every_application_message_has_both_supported_locales() {
        let Value::Object(messages) =
            serde_saphyr::from_str(include_str!("../locales/app.yml")).unwrap()
        else {
            panic!("locale resource must be a mapping");
        };

        for (key, translations) in messages {
            if key == "_version" {
                continue;
            }
            let Value::Object(translations) = translations else {
                panic!("{key} must contain locale values");
            };
            assert!(translations.contains_key("en"), "{key} is missing en");
            assert!(translations.contains_key("zh-CN"), "{key} is missing zh-CN");
            for language in Language::ALL {
                assert_ne!(language.text(&key), key, "{key} is not registered");
            }
        }
    }
}
