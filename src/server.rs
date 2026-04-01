use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};
use tracing::debug;

use crate::api::TwilioClient;
use crate::params::{GetMessageStatusParams, SendMessageParams, SendTemplateParams};

/// The MCP server wrapping the Twilio WhatsApp API.
#[derive(Clone)]
pub struct WhatsAppServer {
    client: std::sync::Arc<TwilioClient>,
    tool_router: ToolRouter<Self>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

impl WhatsAppServer {
    /// Strip the `whatsapp:` prefix if present, then validate E.164 format.
    /// Returns the clean phone number (without prefix).
    fn normalize_phone(number: &str) -> Result<String, McpError> {
        let clean = number.strip_prefix("whatsapp:").unwrap_or(number);
        if !clean.starts_with('+') || clean.len() < 8 || !clean[1..].chars().all(|c| c.is_ascii_digit()) {
            return Err(McpError::invalid_params(
                format!(
                    "Phone number must be in E.164 format (e.g. +27821234567), got \"{number}\""
                ),
                None,
            ));
        }
        Ok(clean.to_string())
    }

    /// Validate a media URL starts with http(s).
    fn validate_media_url(url: &str) -> Result<(), McpError> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(McpError::invalid_params(
                "media_url must start with http:// or https://",
                None,
            ));
        }
        Ok(())
    }

    /// Validate a Twilio Message SID (alphanumeric, starts with "SM").
    fn validate_message_sid(sid: &str) -> Result<(), McpError> {
        if !sid.starts_with("SM") || sid.len() != 34 || !sid.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(McpError::invalid_params(
                format!(
                    "Message SID must be 34 alphanumeric characters starting with \"SM\", got \"{sid}\""
                ),
                None,
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

#[tool_router]
impl WhatsAppServer {
    pub fn new(client: TwilioClient) -> Self {
        Self {
            client: std::sync::Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Send a WhatsApp message via Twilio. Sends a free-form text message \
                        (with optional media attachment) to a phone number. Note: free-form \
                        messages only work within a 24-hour session window after the recipient \
                        has messaged you first, or when using the Twilio sandbox."
    )]
    async fn send_message(
        &self,
        Parameters(p): Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let to = Self::normalize_phone(&p.to)?;
        debug!(to = %to, "send_message tool called");

        if let Some(media) = &p.media_url {
            Self::validate_media_url(media)?;
        }

        match self
            .client
            .send_message(&to, &p.body, p.media_url.as_deref())
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Send a pre-approved WhatsApp template message via Twilio. Template \
                        messages are required to initiate conversations outside the 24-hour \
                        session window. Templates must be created and approved in the Twilio \
                        Console first."
    )]
    async fn send_template(
        &self,
        Parameters(p): Parameters<SendTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let to = Self::normalize_phone(&p.to)?;
        debug!(to = %to, content_sid = %p.content_sid, "send_template tool called");

        if let Some(vars) = &p.content_variables {
            serde_json::from_str::<serde_json::Value>(vars).map_err(|e| {
                McpError::invalid_params(
                    format!("Invalid content_variables JSON: {e}"),
                    None,
                )
            })?;
        }

        match self
            .client
            .send_template(&to, &p.content_sid, p.content_variables.as_deref())
            .await
        {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Check the delivery status of a previously sent WhatsApp message. \
                        Status progresses: queued → sent → delivered → read (or failed/undelivered)."
    )]
    async fn get_message_status(
        &self,
        Parameters(p): Parameters<GetMessageStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        Self::validate_message_sid(&p.message_sid)?;
        debug!(message_sid = %p.message_sid, "get_message_status tool called");

        match self.client.get_message_status(&p.message_sid).await {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }
}

// ---------------------------------------------------------------------------
// MCP ServerHandler
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for WhatsAppServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new(
                "mcp-server-whatsapp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Twilio WhatsApp MCP server. Tools: send_message, send_template, \
                 get_message_status.",
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- normalize_phone ------------------------------------------------------

    #[test]
    fn normalize_phone_valid() {
        assert_eq!(WhatsAppServer::normalize_phone("+27821234567").unwrap(), "+27821234567");
        assert_eq!(WhatsAppServer::normalize_phone("+14155238886").unwrap(), "+14155238886");
    }

    #[test]
    fn normalize_phone_strips_whatsapp_prefix() {
        assert_eq!(
            WhatsAppServer::normalize_phone("whatsapp:+27821234567").unwrap(),
            "+27821234567"
        );
    }

    #[test]
    fn normalize_phone_missing_plus() {
        assert!(WhatsAppServer::normalize_phone("27821234567").is_err());
    }

    #[test]
    fn normalize_phone_too_short() {
        assert!(WhatsAppServer::normalize_phone("+123").is_err());
    }

    #[test]
    fn normalize_phone_non_digits() {
        assert!(WhatsAppServer::normalize_phone("+abcdefgh").is_err());
        assert!(WhatsAppServer::normalize_phone("+2782 1234567").is_err());
    }

    // -- validate_media_url ---------------------------------------------------

    #[test]
    fn validate_media_url_valid() {
        assert!(WhatsAppServer::validate_media_url("https://example.com/img.jpg").is_ok());
        assert!(WhatsAppServer::validate_media_url("http://example.com/img.jpg").is_ok());
    }

    #[test]
    fn validate_media_url_invalid() {
        assert!(WhatsAppServer::validate_media_url("ftp://example.com/img.jpg").is_err());
        assert!(WhatsAppServer::validate_media_url("not-a-url").is_err());
    }

    // -- validate_message_sid -------------------------------------------------

    #[test]
    fn validate_message_sid_valid() {
        // Twilio SIDs are "SM" + 32 hex chars = 34 total
        assert!(WhatsAppServer::validate_message_sid("SM0123456789abcdef0123456789abcdef").is_ok());
    }

    #[test]
    fn validate_message_sid_wrong_prefix() {
        assert!(WhatsAppServer::validate_message_sid("XX0123456789abcdef0123456789abcdef").is_err());
    }

    #[test]
    fn validate_message_sid_wrong_length() {
        assert!(WhatsAppServer::validate_message_sid("SM123").is_err());
    }

    #[test]
    fn validate_message_sid_path_traversal() {
        assert!(WhatsAppServer::validate_message_sid("../../Calls").is_err());
    }
}
