use serde::{Deserialize, Serialize};

use super::{invalid_value, DomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Url,
    TextSnippet,
}

impl ItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::TextSnippet => "text_snippet",
        }
    }
}

impl TryFrom<&str> for ItemKind {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "url" => Ok(Self::Url),
            "text_snippet" => Ok(Self::TextSnippet),
            _ => Err(invalid_value("item_kind", value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSnippetBody(String);

impl TextSnippetBody {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let text = value.into();
        if text.trim().is_empty() {
            return Err(DomainError::EmptyTextSnippet);
        }
        Ok(Self(text))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}
