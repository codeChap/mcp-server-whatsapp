use mcp_server_whatsapp::api::TwilioClient;
use mockito::{Matcher, Server};

fn test_client(base_url: &str) -> TwilioClient {
    TwilioClient::with_base_url(
        "AC_test_sid".into(),
        "test_token".into(),
        "+14155238886".into(),
        base_url.into(),
    )
}

const SEND_RESPONSE: &str = r#"{
    "sid": "SM0123456789abcdef0123456789abcdef",
    "status": "queued",
    "to": "whatsapp:+27821234567",
    "from": "whatsapp:+14155238886",
    "body": "Hello!",
    "date_created": "Mon, 24 Mar 2026 12:00:00 +0000",
    "direction": "outbound-api"
}"#;

#[tokio::test]
async fn send_message_round_trip() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/Messages.json")
        .match_header("authorization", Matcher::Regex("Basic .+".into()))
        .match_body(Matcher::AllOf(vec![
            Matcher::UrlEncoded("From".into(), "whatsapp:+14155238886".into()),
            Matcher::UrlEncoded("To".into(), "whatsapp:+27821234567".into()),
            Matcher::UrlEncoded("Body".into(), "Hello!".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(SEND_RESPONSE)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let resp = client.send_message("+27821234567", "Hello!", None).await.unwrap();

    assert_eq!(resp.sid, "SM0123456789abcdef0123456789abcdef");
    assert_eq!(resp.status.as_deref(), Some("queued"));
    assert_eq!(resp.to.as_deref(), Some("whatsapp:+27821234567"));
    mock.assert_async().await;
}

#[tokio::test]
async fn send_message_with_media() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/Messages.json")
        .match_body(Matcher::AllOf(vec![
            Matcher::UrlEncoded("Body".into(), "Check this".into()),
            Matcher::UrlEncoded("MediaUrl".into(), "https://example.com/img.jpg".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(SEND_RESPONSE)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let resp = client
        .send_message("+27821234567", "Check this", Some("https://example.com/img.jpg"))
        .await
        .unwrap();

    assert_eq!(resp.sid, "SM0123456789abcdef0123456789abcdef");
    mock.assert_async().await;
}

#[tokio::test]
async fn send_template_round_trip() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/Messages.json")
        .match_body(Matcher::AllOf(vec![
            Matcher::UrlEncoded("From".into(), "whatsapp:+14155238886".into()),
            Matcher::UrlEncoded("To".into(), "whatsapp:+27821234567".into()),
            Matcher::UrlEncoded("ContentSid".into(), "HX1234567890abcdef".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(SEND_RESPONSE)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let resp = client
        .send_template("+27821234567", "HX1234567890abcdef", None)
        .await
        .unwrap();

    assert_eq!(resp.sid, "SM0123456789abcdef0123456789abcdef");
    mock.assert_async().await;
}

#[tokio::test]
async fn get_message_status_round_trip() {
    let mut server = Server::new_async().await;
    let sid = "SM0123456789abcdef0123456789abcdef";
    let mock = server
        .mock("GET", format!("/Messages/{sid}.json").as_str())
        .match_header("authorization", Matcher::Regex("Basic .+".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"sid": "SM0123456789abcdef0123456789abcdef", "status": "delivered"}"#)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let resp = client.get_message_status(sid).await.unwrap();

    assert_eq!(resp.status.as_deref(), Some("delivered"));
    mock.assert_async().await;
}

#[tokio::test]
async fn api_error_returns_body() {
    let mut server = Server::new_async().await;
    let mock = server
        .mock("POST", "/Messages.json")
        .with_status(401)
        .with_body(r#"{"message": "Authenticate"}"#)
        .create_async()
        .await;

    let client = test_client(&server.url());
    let err = client
        .send_message("+27821234567", "test", None)
        .await
        .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("401"), "expected 401 in: {msg}");
    assert!(msg.contains("Authenticate"), "expected body in: {msg}");
    mock.assert_async().await;
}
