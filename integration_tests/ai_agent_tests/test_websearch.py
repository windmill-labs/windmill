"""
Websearch tests for AI agents.

Tests that AI agents correctly use the websearch tool:
- OpenAI, Anthropic, and Google AI support websearch
"""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow, create_websearch_tool
from .providers import OPENAI, ANTHROPIC, GOOGLE_AI


def get_provider_ids(providers: list) -> list[str]:
    """Get provider names for pytest parametrization IDs."""
    return [p["name"] for p in providers]


# Only these providers support websearch
WEBSEARCH_PROVIDERS = [
    OPENAI,
    ANTHROPIC,
    GOOGLE_AI,
]


class TestWebsearch:
    """Test AI agent websearch functionality."""

    @pytest.mark.parametrize(
        "provider_config",
        WEBSEARCH_PROVIDERS,
        ids=get_provider_ids(WEBSEARCH_PROVIDERS),
    )
    def test_websearch_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test websearch tool usage.

        The agent should use websearch to find current information and include
        a message indicating successful websearch usage.
        """
        tools = [create_websearch_tool()]

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Use websearch to find current information when asked about recent events or facts that require up-to-date knowledge.",
            tools=tools,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is the current weather in Paris? Please search the web for this information."},
        )

        assert result is not None

        # Verify websearch tool was used
        messages = result.get("messages", [])
        has_websearch_msg = any(
            msg.get("role") == "tool" and
            msg.get("content") == "Used websearch tool successfully"
            for msg in messages
        )
        assert has_websearch_msg, (
            f"Expected 'Used websearch tool successfully' message for {provider_config['name']}: {messages}"
        )

        print(f"Websearch result from {provider_config['name']}: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
