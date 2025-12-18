"""
Tool calling tests for AI agents.

Tests AI agent tool calling with different tool types:
- Rawscript tools (inline Bun/TypeScript)
- MCP tools (external MCP servers)
- Websearch tools (built-in web search)
"""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow, create_rawscript_tool
from .providers import ALL_PROVIDERS


def get_provider_ids(providers: list) -> list[str]:
    """Get provider names for pytest parametrization IDs."""
    return [p["name"] for p in providers]


# Inline script for sum tool (Bun/TypeScript)
ADD_NUMBERS_SCRIPT = """
export function main(a: number, b: number): number {
    return a + b;
}
"""


class TestToolCalling:
    """Test AI agent tool calling with different tool types."""

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_sum_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that an AI agent can call a rawscript tool to add numbers.
        """
        tools = [
            create_rawscript_tool(
                tool_id="add_numbers",
                content=ADD_NUMBERS_SCRIPT,
                params=["a", "b"],
                language="bun",
            )
        ]

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Use the add_numbers tool to perform arithmetic.",
            tools=tools,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 5 + 7? Use the add_numbers tool."},
        )

        assert result is not None
        result_str = str(result)
        assert "12" in result_str, f"Expected '12' in result: {result}"
        print(f"Sum tool result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_mcp_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that an AI agent can call an MCP tool (DeepWiki).
        """
        tools = [
            {
                "id": "deepwiki",
                "value": {
                    "tool_type": "mcp",
                    "resource_path": "$res:u/admin/deepwiki",
                },
            }
        ]

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Use the available tools to answer questions.",
            tools=tools,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Use the read_wiki_structure tool to get the structure of the sveltejs/svelte repository."},
        )

        assert result is not None
        print(f"MCP tool result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_websearch_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that an AI agent can use the websearch tool.
        """
        tools = [
            {
                "id": "websearch",
                "value": {
                    "tool_type": "websearch",
                },
            }
        ]

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Use websearch to find current information.",
            tools=tools,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is the current version of Svelte? Use websearch."},
        )

        assert result is not None
        print(f"Websearch tool result from {provider_config['name']}: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
