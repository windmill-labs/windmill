"""
Tool calling tests for AI agents.

Tests AI agent tool calling with different tool types:
- Rawscript tools (inline Bun/TypeScript)
- Workspace script tools (scripts deployed to the workspace)
- MCP tools (external MCP servers)
- Websearch tools (built-in web search)
"""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow, create_rawscript_tool, create_script_tool
from .providers import ALL_PROVIDERS, ANTHROPIC, GOOGLE_AI, OPENAI, make_provider_input_transform


def get_provider_ids(providers: list) -> list[str]:
    """Get provider names for pytest parametrization IDs."""
    return [p["name"] for p in providers]


# Inline script for sum tool (Bun/TypeScript)
ADD_NUMBERS_SCRIPT = """
export function main(a: number, b: number): number {
    return a + b;
}
"""

GOOGLE_AI_GEMINI_3 = {
    "name": "google_ai_gemini_3",
    "input_transform": make_provider_input_transform(
        kind="googleai",
        model="gemini-3-flash-preview",
        resource_path="u/admin/googleai",
    ),
}


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
    def test_workspace_script_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that an AI agent can call a workspace script tool to add numbers.

        This test uses a script that was deployed to the workspace (u/admin/sum_script)
        rather than an inline rawscript.
        """
        tools = [
            create_script_tool(
                tool_id="sum_numbers",
                script_path="u/admin/sum_script",
                params=["a", "b"],
            )
        ]

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Use the sum_numbers tool to perform arithmetic.",
            tools=tools,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 8 + 15? Use the sum_numbers tool."},
        )

        assert result is not None
        result_str = str(result)
        assert "23" in result_str, f"Expected '23' in result: {result}"
        print(f"Workspace script tool result from {provider_config['name']}: {result}")

    def test_nested_ai_agent_tool_with_gemini_3(
        self,
        client: AIAgentTestClient,
        setup_providers,
    ):
        """
        Test that a Gemini agent can call another Gemini AI agent as a tool.
        """
        nested_ai_agent_tool = {
            "id": "delegate_agent",
            "summary": "delegate_agent",
            "value": {
                "tool_type": "flowmodule",
                "type": "aiagent",
                "input_transforms": {
                    "provider": GOOGLE_AI_GEMINI_3["input_transform"],
                    "system_prompt": {
                        "type": "static",
                        "value": "You are a concise arithmetic helper. Return only the numeric answer.",
                    },
                    "user_message": {"type": "ai"},
                    "output_type": {"type": "static", "value": "text"},
                },
                "tools": [],
            },
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=GOOGLE_AI_GEMINI_3["input_transform"],
            system_prompt="You are a coordinator. Use delegate_agent for arithmetic before answering.",
            tools=[nested_ai_agent_tool],
            output_type="text",
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Ask delegate_agent what 13 + 29 is, then tell me the result."},
        )

        assert result is not None
        result_str = str(result)
        assert "42" in result_str, f"Expected '42' in result: {result}"

        messages = result.get("messages", [])
        assert any(
            tool_call.get("function", {}).get("name") == "delegate_agent"
            for message in messages
            for tool_call in message.get("tool_calls", [])
        ), f"Expected delegate_agent tool call in messages: {messages}"
        print(f"Nested AI agent tool result from {GOOGLE_AI_GEMINI_3['name']}: {result}")

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
        [OPENAI, ANTHROPIC, GOOGLE_AI],
        ids=get_provider_ids([OPENAI, ANTHROPIC, GOOGLE_AI]),
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
