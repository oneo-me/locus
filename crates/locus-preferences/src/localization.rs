use rust_i18n::t;
use serde::{Deserialize, Serialize};

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
            Self::Chinese => t!("language.chinese", locale = self.locale()).into_owned(),
            Self::English => t!("language.english", locale = self.locale()).into_owned(),
        }
    }

    pub fn locale(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }

    pub fn activate(self) {
        gpui_kit::component::set_locale(self.locale());
    }
}
