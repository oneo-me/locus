use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Language {
    Chinese,
    English,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::Chinese, Self::English];

    pub fn label(self) -> &'static str {
        match self {
            Self::Chinese => "中文",
            Self::English => "English",
        }
    }

    pub fn text(self, chinese: &'static str, english: &'static str) -> &'static str {
        match self {
            Self::Chinese => chinese,
            Self::English => english,
        }
    }

    pub fn locale(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::English => "en",
        }
    }
}
