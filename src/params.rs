use schemars::JsonSchema;
use serde::Deserialize;

/// Parameters for the `send_message` tool.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendMessageParams {
    #[schemars(
        description = "Recipient phone number in E.164 format (e.g. \"+27821234567\")."
    )]
    pub to: String,

    #[schemars(description = "The text body of the message to send (text only for now)")]
    pub body: String,
}

/// Parameters for the `send_template` tool.
///
/// Templates are the only way to initiate conversations outside the 24-hour customer service window.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SendTemplateParams {
    #[schemars(
        description = "Recipient phone number in E.164 format (e.g. \"+27821234567\")."
    )]
    pub to: String,

    #[schemars(description = "Name of the pre-approved template (e.g. \"order_confirmation\")")]
    pub template_name: String,

    #[schemars(
        description = "Language code of the template (e.g. \"en_US\", \"en_GB\", \"af\"). Must match how the template was approved."
    )]
    pub language_code: String,

    #[schemars(
        description = "JSON array of template components for variables (body, header, buttons, etc.). \
                       Example for a body with two variables: \
                       [{\"type\":\"body\",\"parameters\":[{\"type\":\"text\",\"text\":\"John\"},{\"type\":\"text\",\"text\":\"tomorrow\"}]}]"
    )]
    pub components: Option<String>,
}
