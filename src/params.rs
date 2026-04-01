use schemars::JsonSchema;
use serde::Deserialize;

/// Parameters for the `send_message` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageParams {
    #[schemars(
        description = "Recipient phone number in E.164 format (e.g. \"+27821234567\"). \
                        The whatsapp: prefix is added automatically."
    )]
    pub to: String,

    #[schemars(description = "The text body of the message to send")]
    pub body: String,

    #[schemars(
        description = "Optional publicly-accessible URL of media to attach (image, PDF, etc.)"
    )]
    pub media_url: Option<String>,
}

/// Parameters for the `send_template` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendTemplateParams {
    #[schemars(
        description = "Recipient phone number in E.164 format (e.g. \"+27821234567\"). \
                        The whatsapp: prefix is added automatically."
    )]
    pub to: String,

    #[schemars(
        description = "The Content SID of the pre-approved template (starts with \"HX\")"
    )]
    pub content_sid: String,

    #[schemars(
        description = "Optional JSON object string mapping template variable placeholders \
                        to values, e.g. {\"1\": \"John\", \"2\": \"tomorrow\"}"
    )]
    pub content_variables: Option<String>,
}

/// Parameters for the `get_message_status` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetMessageStatusParams {
    #[schemars(
        description = "The Message SID returned when the message was sent (starts with \"SM\")"
    )]
    pub message_sid: String,
}
