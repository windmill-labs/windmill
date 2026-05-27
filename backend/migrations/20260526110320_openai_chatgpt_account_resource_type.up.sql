INSERT INTO resource_type(workspace_id, name, schema, description)
SELECT id, 'openai_chatgpt_account', '{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "access_token": {
      "description": "ChatGPT/Codex access token, preferably as a $var reference",
      "type": "string",
      "minLength": 1
    },
    "api_key": {
      "description": "Alias for access_token for compatibility with AI credential handling",
      "type": "string",
      "minLength": 1
    },
    "account_id": {
      "description": "ChatGPT account id from the Codex token claim",
      "type": "string",
      "minLength": 1
    },
    "refresh_token": {
      "description": "ChatGPT/Codex refresh token, preferably as a $var reference",
      "type": "string",
      "minLength": 1
    },
    "expires_at": {
      "description": "RFC3339 access token expiration timestamp",
      "type": "string",
      "minLength": 1
    },
    "auth_mode": {
      "description": "Authentication mode used by the resource",
      "type": "string",
      "default": "chatgpt"
    },
    "client_id": {
      "description": "OAuth client id used for Device Flow",
      "type": "string",
      "minLength": 1
    },
    "base_url": {
      "description": "Optional ChatGPT backend base URL override",
      "type": "string",
      "minLength": 1,
      "default": "https://chatgpt.com/backend-api"
    },
    "headers": {
      "description": "Additional HTTP headers to include in Codex requests",
      "type": "object",
      "additionalProperties": {
        "type": "string"
      }
    }
  },
  "anyOf": [
    { "required": ["access_token"] },
    { "required": ["api_key"] }
  ],
  "required": ["account_id"]
}', 'OpenAI ChatGPT account / Codex authentication resource'
FROM workspace
ON CONFLICT (workspace_id, name) DO NOTHING;
