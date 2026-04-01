use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;
use thiserror::Error;
use tracing::instrument;

/// Errors returned by the Twilio API client.
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Twilio API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

/// Shared HTTP client for all Twilio API calls.
pub struct TwilioClient {
    account_sid: String,
    auth_token: String,
    from_number: String,
    base_url: String,
    http: Client,
}

impl TwilioClient {
    /// Create a new client pointing at the Twilio API.
    pub fn new(account_sid: String, auth_token: String, from_number: String) -> Self {
        Self::with_base_url(
            account_sid.clone(),
            auth_token,
            from_number,
            format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}",
                account_sid
            ),
        )
    }

    /// Create a new client with a custom base URL (useful for testing with mockito).
    pub fn with_base_url(
        account_sid: String,
        auth_token: String,
        from_number: String,
        base_url: String,
    ) -> Self {
        Self {
            account_sid,
            auth_token,
            from_number,
            base_url,
            http: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build reqwest client"),
        }
    }

    /// Check response status and parse JSON, or return an ApiError.
    async fn parse_response(response: reqwest::Response) -> Result<MessageResponse, ApiError> {
        let status = response.status();
        if !status.is_success() {
            let body = match response.text().await {
                Ok(text) => text,
                Err(e) => format!("<failed to read response body: {e}>"),
            };
            tracing::warn!(status = %status, "Twilio API request failed");
            return Err(ApiError::Api { status, body });
        }
        Ok(response.json::<MessageResponse>().await?)
    }

    /// POST a form to the Messages endpoint and parse the response.
    async fn post_message(
        &self,
        params: &HashMap<&str, String>,
    ) -> Result<MessageResponse, ApiError> {
        let endpoint = format!("{}/Messages.json", self.base_url);
        let response = self
            .http
            .post(&endpoint)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(params)
            .send()
            .await?;
        Self::parse_response(response).await
    }

    /// Send a WhatsApp message (free-form text, with optional media URL).
    #[instrument(skip(self, body, media_url), fields(to = %to))]
    pub async fn send_message(
        &self,
        to: &str,
        body: &str,
        media_url: Option<&str>,
    ) -> Result<MessageResponse, ApiError> {
        let mut params = HashMap::new();
        params.insert("From", format!("whatsapp:{}", self.from_number));
        params.insert("To", format!("whatsapp:{}", to));
        params.insert("Body", body.to_string());

        if let Some(media) = media_url {
            params.insert("MediaUrl", media.to_string());
        }

        self.post_message(&params).await
    }

    /// Send a template message (for initiating conversations outside the 24h window).
    #[instrument(skip(self, content_variables), fields(to = %to, content_sid = %content_sid))]
    pub async fn send_template(
        &self,
        to: &str,
        content_sid: &str,
        content_variables: Option<&str>,
    ) -> Result<MessageResponse, ApiError> {
        let mut params = HashMap::new();
        params.insert("From", format!("whatsapp:{}", self.from_number));
        params.insert("To", format!("whatsapp:{}", to));
        params.insert("ContentSid", content_sid.to_string());

        if let Some(vars) = content_variables {
            params.insert("ContentVariables", vars.to_string());
        }

        self.post_message(&params).await
    }

    /// Get the status of a previously sent message.
    #[instrument(skip(self), fields(message_sid = %message_sid))]
    pub async fn get_message_status(
        &self,
        message_sid: &str,
    ) -> Result<MessageResponse, ApiError> {
        let endpoint = format!("{}/Messages/{}.json", self.base_url, message_sid);
        let response = self
            .http
            .get(&endpoint)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .send()
            .await?;
        Self::parse_response(response).await
    }
}

// ---------------------------------------------------------------------------
// Twilio API response types
// ---------------------------------------------------------------------------

/// Response from the Twilio Messages API.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct MessageResponse {
    pub sid: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub date_created: Option<String>,
    #[serde(default)]
    pub date_updated: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub price: Option<String>,
    #[serde(default)]
    pub error_code: Option<i32>,
    #[serde(default)]
    pub error_message: Option<String>,
}

impl fmt::Display for MessageResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "SID: {}", self.sid)?;

        if let Some(status) = &self.status {
            writeln!(f, "Status: {status}")?;
        }
        if let Some(to) = &self.to {
            writeln!(f, "To: {to}")?;
        }
        if let Some(from) = &self.from {
            writeln!(f, "From: {from}")?;
        }
        if let Some(body) = &self.body {
            writeln!(f, "Body: {body}")?;
        }
        if let Some(date) = &self.date_created {
            writeln!(f, "Created: {date}")?;
        }
        if let Some(direction) = &self.direction {
            writeln!(f, "Direction: {direction}")?;
        }
        if let Some(price) = &self.price {
            writeln!(f, "Price: {price}")?;
        }
        if let Some(code) = &self.error_code {
            write!(f, "Error: {code}")?;
            if let Some(msg) = &self.error_message {
                write!(f, " — {msg}")?;
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_message_response_basic() {
        let resp = MessageResponse {
            sid: "SM123".into(),
            status: Some("queued".into()),
            to: Some("whatsapp:+15551234567".into()),
            from: Some("whatsapp:+14155238886".into()),
            body: Some("Hello!".into()),
            date_created: Some("Thu, 24 Mar 2026 12:00:00 +0000".into()),
            date_updated: None,
            direction: Some("outbound-api".into()),
            price: None,
            error_code: None,
            error_message: None,
        };
        let output = resp.to_string();
        assert!(output.contains("SM123"));
        assert!(output.contains("queued"));
        assert!(output.contains("Hello!"));
    }

    #[test]
    fn display_message_response_with_error() {
        let resp = MessageResponse {
            sid: "SM456".into(),
            status: Some("failed".into()),
            to: None,
            from: None,
            body: None,
            date_created: None,
            date_updated: None,
            direction: None,
            price: None,
            error_code: Some(21211),
            error_message: Some("Invalid 'To' phone number".into()),
        };
        let output = resp.to_string();
        assert!(output.contains("21211"));
        assert!(output.contains("Invalid 'To' phone number"));
    }
}
