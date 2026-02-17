"""
AI Provider configurations for testing.

Each provider requires a resource to be set up in the integration-tests workspace.
Resources are created by the setup_providers fixture from environment variables.

Resources created:
- u/admin/openai (openai type)
- u/admin/anthropic (anthropic type)
- u/admin/googleai (googleai type)
- u/admin/openrouter (openrouter type)
- u/admin/bedrock (aws_bedrock type, api key auth)
- u/admin/bedrock_iam (aws_bedrock type, IAM keys auth)
- u/admin/bedrock_env (aws_bedrock type, environment auth)
- u/admin/bedrock_iam_session (aws_bedrock type, IAM session auth)
"""

from typing import Any


def make_provider_input_transform(kind: str, model: str, resource_path: str) -> dict[str, Any]:
    """
    Create an input transform for the provider field in an AI agent module.

    The provider input_transform uses a static value with the ProviderWithResource
    object containing kind, model, and resource reference.
    """
    return {
        "type": "static",
        "value": {
            "kind": kind,
            "model": model,
            "resource": f"$res:{resource_path}",
        },
    }


# Provider configurations
OPENAI = {
    "name": "openai",
    "input_transform": make_provider_input_transform(
        kind="openai",
        model="gpt-4o-mini",
        resource_path="u/admin/openai",
    ),
}

AZURE_OPENAI = {
    "name": "azure_openai",
    "input_transform": make_provider_input_transform(
        kind="azure_openai",
        model="gpt-4o",
        resource_path="u/admin/azure_openai",
    ),
}

ANTHROPIC = {
    "name": "anthropic",
    "input_transform": make_provider_input_transform(
        kind="anthropic",
        model="claude-haiku-4-5-20251001",
        resource_path="u/admin/anthropic",
    ),
}

GOOGLE_AI = {
    "name": "google_ai",
    "input_transform": make_provider_input_transform(
        kind="googleai",
        model="gemini-2.5-flash",
        resource_path="u/admin/googleai",
    ),
}

BEDROCK = {
    "name": "bedrock",
    "input_transform": make_provider_input_transform(
        kind="aws_bedrock",
        model="global.anthropic.claude-haiku-4-5-20251001-v1:0",
        resource_path="u/admin/bedrock",
    ),
}

BEDROCK_API_KEY = {
    "name": "bedrock_api_key",
    "input_transform": make_provider_input_transform(
        kind="aws_bedrock",
        model="global.anthropic.claude-haiku-4-5-20251001-v1:0",
        resource_path="u/admin/bedrock",
    ),
}

BEDROCK_IAM = {
    "name": "bedrock_iam",
    "input_transform": make_provider_input_transform(
        kind="aws_bedrock",
        model="global.anthropic.claude-haiku-4-5-20251001-v1:0",
        resource_path="u/admin/bedrock_iam",
    ),
}

BEDROCK_ENV = {
    "name": "bedrock_env",
    "input_transform": make_provider_input_transform(
        kind="aws_bedrock",
        model="global.anthropic.claude-haiku-4-5-20251001-v1:0",
        resource_path="u/admin/bedrock_env",
    ),
}

BEDROCK_IAM_SESSION = {
    "name": "bedrock_iam_session",
    "input_transform": make_provider_input_transform(
        kind="aws_bedrock",
        model="global.anthropic.claude-haiku-4-5-20251001-v1:0",
        resource_path="u/admin/bedrock_iam_session",
    ),
}

OPENROUTER = {
    "name": "openrouter",
    "input_transform": make_provider_input_transform(
        kind="openrouter",
        model="anthropic/claude-haiku-4.5",
        resource_path="u/admin/openrouter",
    ),
}

# All providers for parametrized tests
ALL_PROVIDERS = [
    OPENAI,
    AZURE_OPENAI,
    ANTHROPIC,
    GOOGLE_AI,
    BEDROCK,
    OPENROUTER,
]

# Vision-capable providers for user_images tests
VISION_PROVIDERS = [
    OPENAI,      # gpt-4o-mini supports vision
    ANTHROPIC,   # claude-3 supports vision
    GOOGLE_AI,   # gemini supports vision
]


def get_provider_ids(providers: list[dict[str, Any]]) -> list[str]:
    """Get provider names for pytest parametrize IDs."""
    return [p["name"] for p in providers]
