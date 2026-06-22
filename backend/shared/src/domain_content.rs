use serde::{Deserialize, Serialize};

use super::{invalid_value, DomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    Url,
    TextSnippet,
    Image,
}

impl ItemKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Url => "url",
            Self::TextSnippet => "text_snippet",
            Self::Image => "image",
        }
    }
}

impl TryFrom<&str> for ItemKind {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "url" => Ok(Self::Url),
            "text_snippet" => Ok(Self::TextSnippet),
            "image" => Ok(Self::Image),
            _ => Err(invalid_value("item_kind", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageUploadStatus {
    Pending,
    Uploaded,
    Failed,
}

impl ImageUploadStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Uploaded => "uploaded",
            Self::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for ImageUploadStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "uploaded" => Ok(Self::Uploaded),
            "failed" => Ok(Self::Failed),
            _ => Err(invalid_value("image_upload_status", value)),
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
