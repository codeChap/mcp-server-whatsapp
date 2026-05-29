use mcp_server_whatsapp::api::MetaClient;
use mockito::Server;

fn test_client(base_url: &str) -> MetaClient {
    MetaClient::with_base_url(
        "test_access_token".into(),
        "123456789012345".into(),
        "v21.0".into(),
        base_url.into(),
    )
}

const SEND_RESPONSE: &str = r#"{
    "messaging_product": "whatsapp",
    "contacts": [{ "input": "+27821234567", "wa_id": "27821234567" }],
    "messages": [{ "id": "wamid.HBgLMTY1MDM4Nzk0MzkVAgASGBQzQUFERjg0NDEzNDdFODU3MUMxMAA=" }]
}"#;

#[tokio::test]
async fn send_message_round_trip() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/messages")
        .match_header("authorization", "Bearer test_access_token")
        .match_body(mockito::Matcher::JsonString(
            r#"{"messaging_product":"whatsapp","recipient_type":"individual","to":"+27821234567","type":"text","text":{"preview_url":false,"body":"Hello!"}}"#.to_string(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SEND_RESPONSE)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let resp = client.send_message("+27821234567", "Hello!").await.unwrap();

    let wamid = resp.messages.as_ref().unwrap()[0].id.as_ref().unwrap();
    assert!(wamid.starts_with("wamid."));
    mock.assert_async().await;
}

#[tokio::test]
async fn send_template_round_trip() {
    let mut server = Server::new_async().await;

    let mock = server
        .mock("POST", "/messages")
        .match_header("authorization", "Bearer test_access_token")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(SEND_RESPONSE)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let components = r#"[{"type":"body","parameters":[{"type":"text","text":"John"}]}]"#;
    let resp = client
        .send_template("+27821234567", "hello_world", "en_US", Some(components))
        .await
        .unwrap();

    assert!(resp.messages.is_some());
    mock.assert_async().await;
}

#[tokio::test]
async fn api_error_returns_meta_error() {
    let mut server = Server::new_async().await;

    let error_body = r#"{
        "error": {
            "message": "Invalid OAuth 2.0 access token",
            "type": "OAuthException",
            "code": 190,
            "fbtrace_id": "AbCdEfGh"
        }
    }"#;

    let mock = server
        .mock("POST", "/messages")
        .with_status(401)
        .with_body(error_body)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let err = client.send_message("+27821234567", "test").await.unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("190"), "expected Meta error code in: {msg}");
    assert!(msg.contains("Invalid OAuth"), "expected message in: {msg}");
    mock.assert_async().await;
}
