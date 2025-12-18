"""
AI Provider configurations for testing.

Each provider requires a resource to be set up in the integration-tests workspace.
Resources are created by the setup_providers fixture from environment variables.

Resources created:
- u/admin/openai (openai type)
- u/admin/anthropic (anthropic type)
- u/admin/googleai (googleai type)
- u/admin/openrouter (openrouter type)
- u/admin/bedrock (aws_bedrock type)
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

ANTHROPIC = {
    "name": "anthropic",
    "input_transform": make_provider_input_transform(
        kind="anthropic",
        model="claude-sonnet-4-20250514",
        resource_path="u/admin/anthropic",
    ),
}

GOOGLE_AI = {
    "name": "google_ai",
    "input_transform": make_provider_input_transform(
        kind="googleai",
        model="gemini-2.0-flash",
        resource_path="u/admin/googleai",
    ),
}

BEDROCK = {
    "name": "bedrock",
    "input_transform": make_provider_input_transform(
        kind="bedrock",
        model="anthropic.claude-3-5-sonnet-20241022-v2:0",
        resource_path="u/admin/bedrock",
    ),
}

OPENROUTER = {
    "name": "openrouter",
    "input_transform": make_provider_input_transform(
        kind="openrouter",
        model="anthropic/claude-sonnet-4",
        resource_path="u/admin/openrouter",
    ),
}

# All providers for parametrized tests
ALL_PROVIDERS = [
    OPENAI,
    ANTHROPIC,
    GOOGLE_AI,
    BEDROCK,
    OPENROUTER,
]
