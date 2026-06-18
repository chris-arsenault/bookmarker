use std::time::Duration;

use async_trait::async_trait;
use thiserror::Error;
use url::Url;

use crate::domain::ArchiveStatus;

const TRACKING_PARAM_PREFIXES: &[&str] = &["utm_"];
const TRACKING_PARAM_NAMES: &[&str] = &[
    "fbclid", "gclid", "gbraid", "wbraid", "mc_cid", "mc_eid", "igshid", "_hsenc", "_hsmi",
];
const TIKTOK_SHORT_HOSTS: &[&str] = &["vt.tiktok.com", "vm.tiktok.com"];
const YOUTUBE_SHORT_HOST: &str = "youtu.be";
const YOUTUBE_WATCH_URL: &str = "https://www.youtube.com/watch";
const HTTP_TIMEOUT: Duration = Duration::from_secs(5);
const HTTP_REDIRECT_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedUrl {
    pub canonical_url: Option<String>,
    pub normalization_status: ArchiveStatus,
    pub normalization_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct ShortUrlResolveError {
    message: String,
}

impl ShortUrlResolveError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[async_trait]
pub trait ShortUrlResolver: Send + Sync {
    async fn resolve(&self, url: &str) -> Result<String, ShortUrlResolveError>;
}

#[derive(Clone)]
pub struct HttpShortUrlResolver {
    client: reqwest::Client,
}

impl HttpShortUrlResolver {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(HTTP_REDIRECT_LIMIT))
            .timeout(HTTP_TIMEOUT)
            .build()
            .expect("HTTP short URL resolver client configuration is valid");
        Self { client }
    }
}

impl Default for HttpShortUrlResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ShortUrlResolver for HttpShortUrlResolver {
    async fn resolve(&self, url: &str) -> Result<String, ShortUrlResolveError> {
        self.client
            .get(url)
            .send()
            .await
            .map(|response| response.url().to_string())
            .map_err(|err| ShortUrlResolveError::new(err.to_string()))
    }
}

impl NormalizedUrl {
    fn succeeded(canonical_url: String) -> Self {
        Self {
            canonical_url: Some(canonical_url),
            normalization_status: ArchiveStatus::Succeeded,
            normalization_error: None,
        }
    }

    fn failed(error: impl Into<String>) -> Self {
        Self {
            canonical_url: None,
            normalization_status: ArchiveStatus::Failed,
            normalization_error: Some(error.into()),
        }
    }
}

pub async fn normalize_url_with_resolver(
    raw_url: &str,
    resolver: &(dyn ShortUrlResolver + Send + Sync),
) -> NormalizedUrl {
    let Ok(parsed) = Url::parse(raw_url) else {
        return NormalizedUrl::failed("submitted URL could not be parsed for normalization");
    };

    if should_resolve_short_url(&parsed) {
        return resolve_short_url(raw_url, resolver).await;
    }

    normalize_parsed_url(parsed)
}

pub fn normalize_url(raw_url: &str) -> NormalizedUrl {
    let Ok(parsed) = Url::parse(raw_url) else {
        return NormalizedUrl::failed("submitted URL could not be parsed for normalization");
    };

    normalize_parsed_url(parsed)
}

fn normalize_parsed_url(parsed: Url) -> NormalizedUrl {
    if is_youtube_short_url(&parsed) {
        return normalize_youtube_short_url(&parsed)
            .map(url_to_success)
            .unwrap_or_else(|| {
                NormalizedUrl::failed("YouTube short URL did not include a video id")
            });
    }

    url_to_success(strip_tracking_params(parsed))
}

async fn resolve_short_url(
    raw_url: &str,
    resolver: &(dyn ShortUrlResolver + Send + Sync),
) -> NormalizedUrl {
    match resolver.resolve(raw_url).await {
        Ok(resolved_url) => normalize_url(&resolved_url),
        Err(err) => NormalizedUrl::failed(format!("short URL resolution failed: {err}")),
    }
}

fn normalize_youtube_short_url(url: &Url) -> Option<Url> {
    let video_id = first_path_segment(url)?;
    let mut canonical = Url::parse(YOUTUBE_WATCH_URL).expect("constant URL is valid");
    {
        let mut query = canonical.query_pairs_mut();
        query.append_pair("v", video_id);
        for (name, value) in url.query_pairs() {
            if !is_tracking_param(&name) {
                query.append_pair(&name, &value);
            }
        }
    }
    Some(canonical)
}

fn strip_tracking_params(mut url: Url) -> Url {
    let retained: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| !is_tracking_param(name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();

    url.set_query(None);
    if !retained.is_empty() {
        let mut query = url.query_pairs_mut();
        for (name, value) in retained {
            query.append_pair(&name, &value);
        }
    }
    url
}

fn is_tracking_param(name: &str) -> bool {
    let normalized = name.to_ascii_lowercase();
    TRACKING_PARAM_NAMES.contains(&normalized.as_str())
        || TRACKING_PARAM_PREFIXES
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
}

fn first_path_segment(url: &Url) -> Option<&str> {
    url.path_segments()?.find(|segment| !segment.is_empty())
}

fn is_youtube_short_url(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case(YOUTUBE_SHORT_HOST))
}

fn should_resolve_short_url(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        TIKTOK_SHORT_HOSTS
            .iter()
            .any(|short_host| host.eq_ignore_ascii_case(short_host))
    })
}

fn url_to_success(url: Url) -> NormalizedUrl {
    NormalizedUrl::succeeded(url.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_youtube_short_links_and_strips_tracking() {
        let youtube = normalize_url(
            "https://youtu.be/video-id?utm_source=share&t=42&list=playlist&utm_campaign=summer",
        );

        assert_eq!(youtube.normalization_status, ArchiveStatus::Succeeded);
        assert_eq!(youtube.normalization_error, None);
        assert_eq!(
            youtube.canonical_url.as_deref(),
            Some("https://www.youtube.com/watch?v=video-id&t=42&list=playlist")
        );

        let regular =
            normalize_url("https://example.com/watch?utm_medium=social&keep=value&fbclid=abc123");

        assert_eq!(regular.normalization_status, ArchiveStatus::Succeeded);
        assert_eq!(regular.normalization_error, None);
        assert_eq!(
            regular.canonical_url.as_deref(),
            Some("https://example.com/watch?keep=value")
        );
    }

    #[tokio::test]
    async fn resolves_known_tiktok_short_links_before_normalizing() {
        let resolver = FakeResolver::succeeds_with(
            "https://www.tiktok.com/@creator/video/123?utm_source=share&is_from_webapp=1",
        );

        let normalized =
            normalize_url_with_resolver("https://vt.tiktok.com/ZSfake/", &resolver).await;

        assert_eq!(normalized.normalization_status, ArchiveStatus::Succeeded);
        assert_eq!(normalized.normalization_error, None);
        assert_eq!(
            normalized.canonical_url.as_deref(),
            Some("https://www.tiktok.com/@creator/video/123?is_from_webapp=1")
        );

        let failed =
            normalize_url_with_resolver("https://vt.tiktok.com/ZSfake/", &FakeResolver::fails())
                .await;

        assert_eq!(failed.normalization_status, ArchiveStatus::Failed);
        assert_eq!(failed.canonical_url, None);
        assert_eq!(
            failed.normalization_error.as_deref(),
            Some("short URL resolution failed: no redirect")
        );
    }

    struct FakeResolver {
        result: Result<String, ShortUrlResolveError>,
    }

    impl FakeResolver {
        fn succeeds_with(url: &str) -> Self {
            Self {
                result: Ok(url.to_string()),
            }
        }

        fn fails() -> Self {
            Self {
                result: Err(ShortUrlResolveError::new("no redirect")),
            }
        }
    }

    #[async_trait]
    impl ShortUrlResolver for FakeResolver {
        async fn resolve(&self, _url: &str) -> Result<String, ShortUrlResolveError> {
            self.result.clone()
        }
    }
}
