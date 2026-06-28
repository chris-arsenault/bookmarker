mod support;

use lambda_http::http::{Method, StatusCode};
use lambda_http::Body;

use support::{
    assert_token_decodes, bearer_token, empty_library, request, response_json, test_app,
};

#[tokio::test]
async fn health_route_returns_service_status_without_auth() {
    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/health",
        None,
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["service"], "linkdrop");
}

#[tokio::test]
async fn me_route_returns_authenticated_user_context() {
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/me",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["sub"], "user-sub");
    assert_eq!(payload["email"], "chris@example.test");
    assert_eq!(payload["username"], "chris");
}

#[tokio::test]
async fn me_route_rejects_missing_auth_metadata() {
    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/me",
        None,
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "unauthorized");
}

#[tokio::test]
async fn api_success_responses_include_cors_header_for_browser_origins() {
    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/health",
        None,
        Body::Empty,
    )
    .await;

    assert_eq!(
        response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
}
