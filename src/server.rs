use rmcp::{
    ErrorData as McpError, ServerHandler, handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters, model::*, tool, tool_handler, tool_router,
};
use tracing::debug;

use crate::api::MetaClient;
use crate::params::{SendMessageParams, SendTemplateParams};

/// The MCP server wrapping the Meta WhatsApp Cloud API.
#[derive(Clone)]
pub struct WhatsAppServer {
    client: std::sync::Arc<MetaClient>,
    tool_router: ToolRouter<Self>,
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

#[tool_router]
impl WhatsAppServer {
    pub fn new(client: MetaClient) -> Self {
        Self {
            client: std::sync::Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

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

    #[tool(
        description = "Send a free-form WhatsApp text message via the Meta Cloud API. \
                        Free-form messages can only be sent inside an active 24-hour customer \
                        service window (after the recipient has messaged you)."
    )]
    async fn send_message(
        &self,
        Parameters(p): Parameters<SendMessageParams>,
    ) -> Result<CallToolResult, McpError> {
        let to = Self::normalize_phone(&p.to)?;
        debug!(to = %to, "send_message tool called");

        match self.client.send_message(&to, &p.body).await {
            Ok(resp) => Ok(CallToolResult::success(vec![Content::text(
                resp.to_string(),
            )])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(e.to_string())])),
        }
    }

    #[tool(
        description = "Send a pre-approved WhatsApp template message via the Meta Cloud API. \
                        Template messages are required to initiate conversations outside the \
                        24-hour customer service window. Templates must be created and approved \
                        in WhatsApp Manager first."
    )]
    async fn send_template(
        &self,
        Parameters(p): Parameters<SendTemplateParams>,
    ) -> Result<CallToolResult, McpError> {
        let to = Self::normalize_phone(&p.to)?;
        debug!(to = %to, template = %p.template_name, "send_template tool called");

        match self
            .client
            .send_template(&to, &p.template_name, &p.language_code, p.components.as_deref())
            .await
        {
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
                "Meta WhatsApp Cloud API MCP server. Tools: send_message, send_template. \
                 Free-form messages only work inside a 24h customer service window. \
                 Use templates to initiate new conversations.",
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

}
