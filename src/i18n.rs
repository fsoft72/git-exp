use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    #[serde(alias = "eng", alias = "ENG", alias = "en", alias = "EN", alias = "English")]
    En,
    #[serde(alias = "ita", alias = "ITA", alias = "it", alias = "IT", alias = "Italiano")]
    It,
}

impl Language {
    pub fn label(self) -> &'static str {
        match self {
            Self::En => "English (ENG)",
            Self::It => "Italiano (ITA)",
        }
    }
}

/// Helper function to select between English and Italian static strings.
#[inline]
pub fn tr<'a>(lang: Language, en: &'a str, it: &'a str) -> &'a str {
    match lang {
        Language::En => en,
        Language::It => it,
    }
}
