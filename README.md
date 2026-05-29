# mcp-server-whatsapp

MCP server for sending WhatsApp messages using the official **Meta WhatsApp Cloud API** (Graph API).

## Features

- `send_message` — Free-form text messages (only works inside an active 24-hour customer service window)
- `send_template` — Send pre-approved template messages (required to start new conversations)
- Full support for template variables via JSON `components`

**Note:** This server no longer uses Twilio. It talks directly to Meta.

## Configuration

Create `~/.config/mcp-server-whatsapp/config.toml`:

```toml
access_token = "EAAxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
phone_number_id = "123456789012345"

# Optional
# api_version = "v21.0"
```

### Getting the credentials

1. Go to [Meta for Developers](https://developers.facebook.com/) → Your App → WhatsApp → API Setup
2. Create a **System User** in Business Settings with the `whatsapp_business_messaging` permission
3. Generate a long-lived access token
4. Copy the **Phone number ID** (not the actual phone number)

## Usage (MCP)

The server exposes these tools:

### send_message

```json
{
  "to": "+27821234567",
  "body": "Hello from the MCP server!"
}
```

### send_template

```json
{
  "to": "+27821234567",
  "template_name": "order_confirmation",
  "language_code": "en_US",
  "components": "[{\"type\":\"body\",\"parameters\":[{\"type\":\"text\",\"text\":\"John\"},{\"type\":\"text\",\"text\":\"tomorrow\"}]}]"
}
```

## Migration from the old Twilio version

The previous version used Twilio. The new config format is completely different:

| Old (Twilio)          | New (Meta)             |
|-----------------------|------------------------|
| `account_sid`         | `access_token`         |
| `auth_token`          | (part of access_token) |
| `from_number`         | `phone_number_id`      |
| `content_sid` (HX...) | `template_name` + `language_code` |

**`get_message_status` has been removed.** Meta does not provide a synchronous lookup endpoint. Every successful send returns a `wamid` that you can correlate with webhook events.

## Future work

- Media upload tool (`upload_media`) to get Meta media IDs for rich messages and template headers
- Support for more message types (interactive, location, etc.)
- Template management helpers

## Development

```bash
cargo test
cargo build --release
```

The binary is `mcp-server-whatsapp`.

## License

MIT (or whatever you prefer)
