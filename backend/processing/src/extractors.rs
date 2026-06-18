use async_trait::async_trait;
use scraper::{Html, Selector};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSource {
    pub url: String,
}

impl MetadataSource {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedMetadata {
    pub title: Option<String>,
    pub thumbnail_url: Option<String>,
    pub author: Option<String>,
    pub platform: Option<String>,
    pub duration_seconds: Option<i32>,
}

impl ExtractedMetadata {
    fn has_metadata(&self) -> bool {
        self.title.is_some()
            || self.thumbnail_url.is_some()
            || self.author.is_some()
            || self.platform.is_some()
            || self.duration_seconds.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct FetchResponse {
    pub body: String,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum MetadataFetchError {
    #[error("metadata fetch failed for {0}")]
    FetchFailed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum MetadataExtractionError {
    #[error(transparent)]
    Fetch(#[from] MetadataFetchError),
    #[error("no metadata found")]
    NoMetadataFound,
}

#[async_trait]
pub trait MetadataFetch: Send + Sync {
    async fn fetch_text(&self, url: &str) -> Result<FetchResponse, MetadataFetchError>;
}

#[derive(Clone)]
pub struct ReqwestMetadataFetch {
    client: reqwest::Client,
}

impl ReqwestMetadataFetch {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl MetadataFetch for ReqwestMetadataFetch {
    async fn fetch_text(&self, url: &str) -> Result<FetchResponse, MetadataFetchError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| MetadataFetchError::FetchFailed(err.to_string()))?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let body = response
            .text()
            .await
            .map_err(|err| MetadataFetchError::FetchFailed(err.to_string()))?;
        Ok(FetchResponse { body, content_type })
    }
}

pub struct OpenGraphExtractor<F> {
    fetcher: F,
}

impl<F> OpenGraphExtractor<F>
where
    F: MetadataFetch,
{
    pub fn new(fetcher: F) -> Self {
        Self { fetcher }
    }

    pub async fn extract(
        &self,
        source: MetadataSource,
    ) -> Result<ExtractedMetadata, MetadataExtractionError> {
        if let Some(provider_url) = provider_oembed_url(&source.url) {
            if let Ok(response) = self.fetcher.fetch_text(&provider_url).await {
                let metadata = parse_oembed(&response.body);
                if metadata.has_metadata() {
                    return Ok(metadata);
                }
            }
        }

        let response = self.fetcher.fetch_text(&source.url).await?;
        let metadata = parse_html_metadata(&source.url, &response.body);
        if metadata.has_metadata() {
            Ok(metadata)
        } else {
            Err(MetadataExtractionError::NoMetadataFound)
        }
    }
}

fn provider_oembed_url(source_url: &str) -> Option<String> {
    let parsed = Url::parse(source_url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    let encoded_url = encode_query_value(source_url);
    if host == "youtu.be" || host.ends_with("youtube.com") {
        return Some(format!(
            "https://www.youtube.com/oembed?url={encoded_url}&format=json"
        ));
    }
    if host.ends_with("tiktok.com") {
        return Some(format!("https://www.tiktok.com/oembed?url={encoded_url}"));
    }
    None
}

fn encode_query_value(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}

#[derive(Debug, Deserialize)]
struct OembedResponse {
    title: Option<String>,
    author_name: Option<String>,
    provider_name: Option<String>,
    thumbnail_url: Option<String>,
}

fn parse_oembed(body: &str) -> ExtractedMetadata {
    let parsed = serde_json::from_str::<OembedResponse>(body).ok();
    ExtractedMetadata {
        title: parsed
            .as_ref()
            .and_then(|body| clean(body.title.as_deref())),
        thumbnail_url: parsed
            .as_ref()
            .and_then(|body| clean(body.thumbnail_url.as_deref())),
        author: parsed
            .as_ref()
            .and_then(|body| clean(body.author_name.as_deref())),
        platform: parsed
            .as_ref()
            .and_then(|body| clean(body.provider_name.as_deref())),
        duration_seconds: None,
    }
}

fn parse_html_metadata(source_url: &str, body: &str) -> ExtractedMetadata {
    let document = Html::parse_document(body);
    ExtractedMetadata {
        title: select_meta(&document, "property", &["og:title", "twitter:title"])
            .or_else(|| select_title(&document)),
        thumbnail_url: select_meta(&document, "property", &["og:image", "twitter:image"])
            .or_else(|| select_meta(&document, "name", &["twitter:image"])),
        author: select_meta(&document, "name", &["author"])
            .or_else(|| select_meta(&document, "property", &["article:author"])),
        platform: select_meta(&document, "property", &["og:site_name"])
            .or_else(|| platform_from_url(source_url)),
        duration_seconds: select_meta(
            &document,
            "property",
            &["video:duration", "og:video:duration"],
        )
        .and_then(|duration| duration.parse::<i32>().ok()),
    }
}

fn select_meta(document: &Html, attribute: &str, names: &[&str]) -> Option<String> {
    for name in names {
        let selector = Selector::parse(&format!("meta[{attribute}=\"{name}\"]")).ok()?;
        let value = document
            .select(&selector)
            .find_map(|element| element.value().attr("content"))
            .and_then(|value| clean(Some(value)));
        if value.is_some() {
            return value;
        }
    }
    None
}

fn select_title(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").ok()?;
    document
        .select(&selector)
        .find_map(|element| clean(Some(&element.text().collect::<String>())))
}

fn platform_from_url(source_url: &str) -> Option<String> {
    let host = Url::parse(source_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))?;
    clean(Some(host.trim_start_matches("www.")))
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{
        FetchResponse, MetadataFetch, MetadataFetchError, MetadataSource, OpenGraphExtractor,
    };
    use async_trait::async_trait;
    use std::collections::HashMap;

    #[tokio::test]
    async fn extracts_opengraph_video_metadata() {
        let fetcher = FakeFetch::new().with(
            "https://example.com/watch/1",
            r#"
            <html>
              <head>
                <meta property="og:title" content="Saved video">
                <meta property="og:image" content="https://cdn.example.test/thumb.jpg">
                <meta property="og:site_name" content="Example Video">
                <meta name="author" content="Creator Name">
                <meta property="video:duration" content="123">
              </head>
            </html>
            "#,
            "text/html",
        );
        let extractor = OpenGraphExtractor::new(fetcher);

        let metadata = extractor
            .extract(MetadataSource::new("https://example.com/watch/1"))
            .await
            .unwrap();

        assert_eq!(metadata.title.as_deref(), Some("Saved video"));
        assert_eq!(
            metadata.thumbnail_url.as_deref(),
            Some("https://cdn.example.test/thumb.jpg")
        );
        assert_eq!(metadata.author.as_deref(), Some("Creator Name"));
        assert_eq!(metadata.platform.as_deref(), Some("Example Video"));
        assert_eq!(metadata.duration_seconds, Some(123));
    }

    #[tokio::test]
    async fn extracts_provider_oembed_metadata() {
        let fetcher = FakeFetch::new().with(
            "https://www.youtube.com/oembed?url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3Dvideo-id&format=json",
            r#"{
                "title": "Provider title",
                "author_name": "Provider creator",
                "provider_name": "YouTube",
                "thumbnail_url": "https://i.ytimg.com/vi/video-id/hqdefault.jpg"
            }"#,
            "application/json",
        );
        let extractor = OpenGraphExtractor::new(fetcher);

        let metadata = extractor
            .extract(MetadataSource::new(
                "https://www.youtube.com/watch?v=video-id",
            ))
            .await
            .unwrap();

        assert_eq!(metadata.title.as_deref(), Some("Provider title"));
        assert_eq!(metadata.author.as_deref(), Some("Provider creator"));
        assert_eq!(metadata.platform.as_deref(), Some("YouTube"));
        assert_eq!(
            metadata.thumbnail_url.as_deref(),
            Some("https://i.ytimg.com/vi/video-id/hqdefault.jpg")
        );
    }

    struct FakeFetch {
        responses: HashMap<String, FetchResponse>,
    }

    impl FakeFetch {
        fn new() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }

        fn with(mut self, url: &str, body: &str, content_type: &str) -> Self {
            self.responses.insert(
                url.to_string(),
                FetchResponse {
                    body: body.to_string(),
                    content_type: Some(content_type.to_string()),
                },
            );
            self
        }
    }

    #[async_trait]
    impl MetadataFetch for FakeFetch {
        async fn fetch_text(&self, url: &str) -> Result<FetchResponse, MetadataFetchError> {
            self.responses
                .get(url)
                .cloned()
                .ok_or_else(|| MetadataFetchError::FetchFailed(url.to_string()))
        }
    }
}
