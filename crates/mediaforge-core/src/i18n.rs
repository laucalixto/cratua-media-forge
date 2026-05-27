use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en-US")]
    EnUs,
    #[serde(rename = "pt-BR")]
    PtBr,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::EnUs => "en-US",
            Language::PtBr => "pt-BR",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Language::EnUs => "English",
            Language::PtBr => "Português (BR)",
        }
    }

    pub const ALL: &[Language] = &[Language::EnUs, Language::PtBr];
}
