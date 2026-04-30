use codex_image::diagnostics::{CliError, ExitCode};
use codex_image::openai::{generate_image, ImageGenerationRequest, GPT_IMAGE_MODEL};
use reqwest::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn build_request(prompt: &str) -> ImageGenerationRequest {
    ImageGenerationRequest {
        prompt: prompt.to_string(),
        size: Some("1024x1024".to_string()),
        quality: Some("high".to_string()),
        background: Some("transparent".to_string()),
        output_format: None,
    }
}

fn base_url(server: &MockServer, trailing_slash: bool) -> Url {
    let mut uri = server.uri();
    if trailing_slash {
        uri.push('/');
    }
    Url::parse(&uri).expect("mock URI must parse")
}

#[tokio::test]
async fn openai_generate_posts_expected_request_shape_and_parses_response() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .and(header("authorization", "Bearer access-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1_745_999_111,
            "data": [
                {
                    "b64_json": "ZmFrZS1iYXNlNjQ=",
                    "revised_prompt": "revised prompt",
                    "size": "1024x1024",
                    "quality": "high",
                    "background": "transparent",
                    "output_format": "png"
                }
            ],
            "usage": {
                "total_tokens": 42,
                "input_tokens": 40,
                "output_tokens": 2
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("sunrise over mountains");

    let result = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect("request should succeed");

    assert_eq!(result.created, 1_745_999_111);
    assert_eq!(result.data.len(), 1);
    assert_eq!(result.data[0].b64_json, "ZmFrZS1iYXNlNjQ=");
    assert_eq!(
        result.data[0].revised_prompt.as_deref(),
        Some("revised prompt")
    );
    assert_eq!(result.data[0].size.as_deref(), Some("1024x1024"));
    assert_eq!(result.data[0].quality.as_deref(), Some("high"));
    assert_eq!(result.data[0].background.as_deref(), Some("transparent"));
    assert_eq!(result.data[0].output_format.as_deref(), Some("png"));

    let usage = result.usage.expect("usage should be present");
    assert_eq!(usage.total_tokens, Some(42));
    assert_eq!(usage.input_tokens, Some(40));
    assert_eq!(usage.output_tokens, Some(2));

    let requests = server
        .received_requests()
        .await
        .expect("received requests should be available");
    assert_eq!(requests.len(), 1);

    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).expect("request body should be JSON");
    assert_eq!(body["model"], GPT_IMAGE_MODEL);
    assert_eq!(body["prompt"], "sunrise over mountains");
    assert_eq!(body["output_format"], "png");
    assert_eq!(body["size"], "1024x1024");
    assert_eq!(body["quality"], "high");
    assert_eq!(body["background"], "transparent");
    assert!(
        body.get("response_format").is_none(),
        "response_format must not be sent"
    );
}

#[tokio::test]
async fn openai_generate_non_2xx_maps_to_api_diagnostic_without_leaking_body() {
    let server = MockServer::start().await;
    let sentinel = "Bearer access-token sk-live-secret prompt-body b64_json";

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(500).set_body_string(sentinel))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("sensitive prompt text");

    let err = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect_err("500 should fail");

    assert!(matches!(err, CliError::ImageGenerationApi { .. }));
    assert_eq!(err.exit_code(), ExitCode::Api);

    let envelope = serde_json::to_string(&err.error_envelope()).unwrap();
    assert!(!envelope.contains("Bearer"));
    assert!(!envelope.contains("access-token"));
    assert!(!envelope.contains("sk-live-secret"));
    assert!(!envelope.contains("prompt-body"));
    assert!(!envelope.contains("b64_json"));
    assert_eq!(err.redacted_message(), "image generation request failed");

    if let CliError::ImageGenerationApi { source_message } = err {
        assert!(!source_message.contains("Bearer"));
        assert!(!source_message.contains("access-token"));
        assert!(!source_message.contains("sk-live-secret"));
        assert!(!source_message.contains("prompt-body"));
        assert!(!source_message.contains("b64_json"));
    }
}

#[tokio::test]
async fn openai_generate_missing_image_scope_maps_to_auth_diagnostic_without_leaking_body() {
    let server = MockServer::start().await;
    let upstream_body = serde_json::json!({
        "error": "You have insufficient permissions for this operation. Missing scopes: api.model.images.request. Bearer access-token sk-live-secret prompt-body b64_json"
    });

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(401).set_body_json(upstream_body))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("sensitive prompt text");

    let err = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect_err("missing image scope should fail");

    assert!(matches!(err, CliError::AuthInsufficientScope));
    assert_eq!(err.exit_code(), ExitCode::Auth);

    let envelope = serde_json::to_string(&err.error_envelope()).unwrap();
    assert!(envelope.contains("auth.insufficient_scope"));
    assert!(!envelope.contains("Bearer"));
    assert!(!envelope.contains("access-token"));
    assert!(!envelope.contains("sk-live-secret"));
    assert!(!envelope.contains("prompt-body"));
    assert!(!envelope.contains("b64_json"));
}

#[tokio::test]
async fn openai_generate_malformed_json_maps_to_response_contract() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not-json"))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("a prompt");

    let err = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect_err("invalid JSON should fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
    assert_eq!(err.exit_code(), ExitCode::ResponseContract);
}

#[tokio::test]
async fn openai_generate_empty_data_maps_to_response_contract() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("a prompt");

    let err = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect_err("empty data should fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
    assert_eq!(err.exit_code(), ExitCode::ResponseContract);
}

#[tokio::test]
async fn openai_generate_missing_or_empty_b64_json_maps_to_response_contract() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/images/generations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "created": 1,
            "data": [
                {},
                {"b64_json": "   "}
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let client = reqwest::Client::new();
    let request = build_request("a prompt");

    let err = generate_image(&client, &base_url(&server, false), "access-token", &request)
        .await
        .expect_err("missing/empty b64_json should fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationResponseContract { .. }
    ));
    assert_eq!(err.exit_code(), ExitCode::ResponseContract);
}

#[tokio::test]
async fn openai_generate_base_url_path_joining_handles_trailing_and_non_trailing_slash() {
    let client = reqwest::Client::new();

    for trailing_slash in [false, true] {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1,
                "data": [{"b64_json": "Zm9v"}]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let request = build_request("a prompt");
        let result = generate_image(
            &client,
            &base_url(&server, trailing_slash),
            "access-token",
            &request,
        )
        .await;

        assert!(
            result.is_ok(),
            "request should succeed for trailing_slash={trailing_slash}"
        );
    }
}

#[tokio::test]
async fn openai_generate_transport_failure_maps_to_api_failure_category() {
    let client = reqwest::Client::new();
    let request = build_request("a prompt");
    let invalid_base = Url::parse("http://127.0.0.1:1").unwrap();

    let err = generate_image(&client, &invalid_base, "access-token", &request)
        .await
        .expect_err("connection error should fail");

    assert!(matches!(
        err,
        CliError::ImageGenerationApi { .. } | CliError::ImageGenerationTimeout { .. }
    ));
    assert_eq!(err.exit_code(), ExitCode::Api);
    assert_eq!(
        err.error_envelope().error.code,
        "api.image_generation_failed"
    );
    assert_eq!(err.exit_code().as_i32(), 4);
}
