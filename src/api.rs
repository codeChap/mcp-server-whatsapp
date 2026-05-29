use reqwest::Client;
use serde::Deserialize;
use std::fmt;
use std::time::Duration;
use thiserror::Error;
use tracing::instrument;

/// Errors returned by the Meta Graph API client.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Meta API error (code {code}): {message}")]
    Meta {
        code: i64,
        message: String,
        raw: String,
    },

    #[error("Meta API error ({status}): {body}")]
    Http { status: reqwest::StatusCode, body: String },
}

/// Shared HTTP client for Meta WhatsApp Cloud API calls.
pub struct MetaClient {
    access_token: String,
    #[allow(dead_code)]
    phone_number_id: String,
    #[allow(dead_code)]
    api_version: String,
    base_url: String,
    http: Client,
}

impl MetaClient {
    /// Create a new client for the Meta WhatsApp Cloud API.
    pub fn new(access_token: String, phone_number_id: String, api_version: String) -> Self {
        let base_url = format!(
            "https://graph.facebook.com/{}/{}",
            api_version, phone_number_id
        );
        Self {
            access_token,
            phone_number_id,
            api_version,
            base_url,
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// Create a client with a custom base URL (for testing with mockito).
    #[allow(dead_code)]
    pub fn with_base_url(
        access_token: String,
        phone_number_id: String,
        api_version: String,
        base_url: String,
    ) -> Self {
        Self {
            access_token,
            phone_number_id,
            api_version,
            base_url,
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// Parse a Meta Graph API response. Handles both success and the nested error shape.
    async fn parse_response(response: reqwest::Response) -> Result<MessageResponse, ApiError> {
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();

            // Try to extract the nice Meta error shape
            if let Ok(err_wrapper) = serde_json::from_str::<MetaErrorWrapper>(&body) {
                if let Some(err) = err_wrapper.error {
                    return Err(ApiError::Meta {
                        code: err.code,
                        message: err.message,
                        raw: body,
                    });
                }
            }

            tracing::warn!(status = %status, "Meta Graph API request failed");
            return Err(ApiError::Http { status, body });
        }

        Ok(response.json::<MessageResponse>().await?)
    }

    /// Send a free-form text message (only allowed inside 24h customer service window).
    #[instrument(skip(self, body), fields(to = %to))]
    pub async fn send_message(&self, to: &str, body: &str) -> Result<MessageResponse, ApiError> {
        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": body
            }
        });

        let endpoint = format!("{}/messages", self.base_url);
        let response = self
            .http
            .post(&endpoint)
            .bearer_auth(&self.access_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        Self::parse_response(response).await
    }

    /// Send a pre-approved template message.
    ///
    /// TODO: Add an `upload_media` method + tool in the future for header media and document messages.
    ///
    /// `components_json` is an optional JSON string containing the components array
    /// (for body parameters, header media, buttons, etc.).
    #[instrument(skip(self, components_json), fields(to = %to, template = %template_name))]
    pub async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        language_code: &str,
        components_json: Option<&str>,
    ) -> Result<MessageResponse, ApiError> {
        let mut template = serde_json::json!({
            "name": template_name,
            "language": { "code": language_code }
        });

        if let Some(components_str) = components_json {
            if !components_str.trim().is_empty() {
                let components: serde_json::Value = serde_json::from_str(components_str)
                    .map_err(|e| ApiError::Http {
                        status: reqwest::StatusCode::BAD_REQUEST,
                        body: format!("Invalid components JSON: {e}"),
                    })?;
                template["components"] = components;
            }
        }

        let payload = serde_json::json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "template",
            "template": template
        });

        let endpoint = format!("{}/messages", self.base_url);
        let response = self
            .http
            .post(&endpoint)
            .bearer_auth(&self.access_token)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        Self::parse_response(response).await
    }
}

// ---------------------------------------------------------------------------
// Meta Graph API response & error types
// ---------------------------------------------------------------------------

/// Top-level response from POST /messages
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageResponse {
    #[serde(default)]
    pub messaging_product: Option<String>,
    #[serde(default)]
    pub contacts: Option<Vec<Contact>>,
    #[serde(default)]
    pub messages: Option<Vec<MessageId>>,
}

#[derive(Debug, Deserialize)]
pub struct Contact {
    #[serde(default)]
    #[allow(dead_code)]
    pub input: Option<String>,
    #[serde(default)]
    pub wa_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MessageId {
    #[serde(default)]
    pub id: Option<String>,
}

impl fmt::Display for MessageResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(msgs) = &self.messages {
            if let Some(first) = msgs.first() {
                if let Some(id) = &first.id {
                    writeln!(f, "Message ID (wamid): {}", id)?;
                }
            }
        }
        if let Some(contacts) = &self.contacts {
            if let Some(c) = contacts.first() {
                if let Some(wa_id) = &c.wa_id {
                    writeln!(f, "Recipient wa_id: {}", wa_id)?;
                }
            }
        }
        Ok(())
    }
}

/// Wrapper for Meta's error responses: { "error": { ... } }
#[derive(Debug, Deserialize)]
struct MetaErrorWrapper {
    error: Option<MetaErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct MetaErrorDetail {
    message: String,
    code: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_response_basic() {
        let resp = MessageResponse {
            messaging_product: Some("whatsapp".into()),
            contacts: Some(vec![Contact {
                input: Some("+27821234567".into()),
                wa_id: Some("27821234567".into()),
            }]),
            messages: Some(vec![MessageId {
                id: Some("wamid.HBgLMTY1MDM4Nzk0MzkVAgASGBQzQUFERjg0NDEzNDdFODU3MUMxMAA=".into()),
            }]),
        };
        let output = resp.to_string();
        assert!(output.contains("wamid.HBgL"));
        assert!(output.contains("27821234567"));
    }
}
