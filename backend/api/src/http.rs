use std::borrow::Cow;

use lambda_http::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CACHE_CONTROL, CONTENT_TYPE,
};
use lambda_http::http::{HeaderValue, Method, StatusCode};
use lambda_http::{Body, Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;

pub(crate) trait PublicHttpError {
    fn status_code(&self) -> StatusCode;
    fn code(&self) -> Cow<'_, str>;
    fn message(&self) -> Cow<'_, str>;
}

#[derive(Debug, Clone)]
pub(crate) struct HttpError {
    status_code: StatusCode,
    code: Cow<'static, str>,
    message: Cow<'static, str>,
}

impl HttpError {
    pub(crate) fn bad_request(message: impl Into<Cow<'static, str>>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    pub(crate) fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", "not found")
    }

    fn new(
        status_code: StatusCode,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            status_code,
            code: code.into(),
            message: message.into(),
        }
    }
}

impl PublicHttpError for HttpError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn code(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.code.as_ref())
    }

    fn message(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.message.as_ref())
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Route<'a> {
    method: &'a Method,
    path: &'a str,
}

impl<'a> Route<'a> {
    pub(crate) fn from_request(request: &'a Request) -> Self {
        Self {
            method: request.method(),
            path: request.uri().path(),
        }
    }

    pub(crate) fn matches(
        &self,
        method: Method,
        pattern: impl AsRef<str>,
    ) -> Result<Option<PathParams>, HttpError> {
        if self.method != method {
            return Ok(None);
        }
        match_segments(split_segments(pattern.as_ref()), split_segments(self.path))
    }

    pub(crate) fn is_match(
        &self,
        method: Method,
        pattern: impl AsRef<str>,
    ) -> Result<bool, HttpError> {
        self.matches(method, pattern)
            .map(|matched| matched.is_some())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PathParams {
    values: Vec<(String, String)>,
}

impl PathParams {
    fn new(values: Vec<(String, String)>) -> Self {
        Self { values }
    }

    pub(crate) fn parse<T>(&self, name: &str) -> Result<T, HttpError>
    where
        T: std::str::FromStr,
        T::Err: std::fmt::Display,
    {
        let value = self
            .values
            .iter()
            .find_map(|(key, value)| (key == name).then_some(value.as_str()))
            .ok_or_else(|| HttpError::bad_request(format!("missing path parameter: {name}")))?;
        value.parse::<T>().map_err(|error| {
            HttpError::bad_request(format!("invalid path parameter {name}: {error}"))
        })
    }
}

pub(crate) fn default_cors(mut response: Response<Body>) -> Response<Body> {
    let headers = response.headers_mut();
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET,POST,PUT,PATCH,DELETE,HEAD,OPTIONS"),
    );
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("*"));
    response
}

pub(crate) fn body_bytes(body: &Body) -> &[u8] {
    match body {
        Body::Empty => &[],
        Body::Text(value) => value.as_bytes(),
        Body::Binary(value) => value.as_slice(),
    }
}

pub(crate) fn json_body<T: DeserializeOwned>(request: &Request) -> Result<T, HttpError> {
    let body = body_bytes(request.body());
    if body.is_empty() {
        return Err(HttpError::bad_request("request body is required"));
    }
    serde_json::from_slice(body)
        .map_err(|error| HttpError::bad_request(format!("invalid JSON body: {error}")))
}

pub(crate) fn query_params<T: DeserializeOwned>(request: &Request) -> Result<T, HttpError> {
    serde_urlencoded::from_str(request.uri().query().unwrap_or_default())
        .map_err(|error| HttpError::bad_request(format!("invalid query string: {error}")))
}

pub(crate) fn json_response<T: Serialize>(
    status: StatusCode,
    value: &T,
) -> Result<Response<Body>, HttpError> {
    let body = serde_json::to_string(value).map_err(|_| {
        HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "failed to serialize response body",
        )
    })?;
    response_with_body(status, "application/json", Body::Text(body))
}

pub(crate) fn json_value_response(status: StatusCode, value: serde_json::Value) -> Response<Body> {
    response_with_body(status, "application/json", Body::Text(value.to_string()))
        .expect("valid JSON response")
}

pub(crate) fn binary_response(
    status: StatusCode,
    content_type: impl AsRef<str>,
    bytes: impl Into<Vec<u8>>,
) -> Result<Response<Body>, HttpError> {
    response_with_body(status, content_type, Body::Binary(bytes.into()))
}

pub(crate) fn no_content_response() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Body::Empty)
        .expect("valid empty response")
}

pub(crate) fn error_response(error: &impl PublicHttpError) -> Response<Body> {
    json_value_response(
        error.status_code(),
        json!({
            "code": error.code(),
            "message": error.message(),
        }),
    )
}

pub(crate) fn private_immutable_cache(mut response: Response<Body>) -> Response<Body> {
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=31536000, immutable"),
    );
    response
}

fn match_segments(pattern: Vec<&str>, path: Vec<&str>) -> Result<Option<PathParams>, HttpError> {
    if pattern.len() != path.len() {
        return Ok(None);
    }
    let mut values = Vec::new();
    for (pattern_segment, path_segment) in pattern.into_iter().zip(path) {
        if let Some(param_name) = param_name(pattern_segment)? {
            values.push((param_name.to_string(), path_segment.to_string()));
        } else if pattern_segment != path_segment {
            return Ok(None);
        }
    }
    Ok(Some(PathParams::new(values)))
}

fn split_segments(path: &str) -> Vec<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

fn param_name(segment: &str) -> Result<Option<&str>, HttpError> {
    if !segment.starts_with('{') && !segment.ends_with('}') {
        return Ok(None);
    }
    if segment.starts_with('{') && segment.ends_with('}') {
        let name = &segment[1..segment.len() - 1];
        if !name.is_empty() && !name.starts_with('*') {
            return Ok(Some(name));
        }
    }
    Err(HttpError::new(
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal_error",
        format!("invalid route parameter segment: {segment}"),
    ))
}

fn response_with_body(
    status: StatusCode,
    content_type: impl AsRef<str>,
    body: Body,
) -> Result<Response<Body>, HttpError> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, header_value(content_type.as_ref())?)
        .body(body)
        .map_err(|_| {
            HttpError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_error",
                "failed to build response",
            )
        })
}

fn header_value(value: &str) -> Result<HeaderValue, HttpError> {
    HeaderValue::from_str(value).map_err(|_| {
        HttpError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "invalid response header value",
        )
    })
}

pub(crate) mod prelude {
    pub(crate) use super::{
        binary_response, json_body, json_response, json_value_response, no_content_response,
        private_immutable_cache, query_params, Route,
    };
    pub(crate) use lambda_http::http::{header, HeaderMap, Method, StatusCode};
    pub(crate) use lambda_http::Request;
}
