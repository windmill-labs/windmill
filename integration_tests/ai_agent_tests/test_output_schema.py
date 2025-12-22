"""
Output schema tests for AI agents.

Tests that AI agents correctly handle structured output with output_schema:
- With a tool (agent uses tool then returns structured output)
- Without a tool (agent returns structured output directly)
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

# Output schema for structured result
RESULT_SCHEMA = {
    "type": "object",
    "properties": {
        "sum": {"type": "number"}
    },
    "required": ["sum"]
}


class TestOutputSchema:
    """Test AI agent output_schema handling."""

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_output_schema_with_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test output_schema with a tool.

        The agent should use the add_numbers tool and return structured output.
        """
        if provider_config["name"] == "google_ai":
            pytest.xfail("Google AI does not support output_schema with tools")

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
            system_prompt="You are a helpful assistant. Use the add_numbers tool to perform arithmetic. Return the result in the structured format.",
            tools=tools,
            output_schema=RESULT_SCHEMA,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 5 + 7? Use the add_numbers tool."},
        )

        assert result is not None
        # The result should be structured with a "result" field
        assert "sum" in result or "12" in str(result), f"Expected structured result with '12': {result}"

        # For Anthropic and Bedrock, verify structured_output tool was used
        if provider_config["name"] in ("anthropic", "bedrock"):
            messages = result.get("messages", [])
            has_structured_output_msg = any(
                msg.get("role") == "tool" and
                msg.get("content") == "Successfully ran structured_output tool"
                for msg in messages
            )
            assert has_structured_output_msg, (
                f"Expected 'Successfully ran structured_output tool' message for {provider_config['name']}: {messages}"
            )

        print(f"Output schema with tool result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_output_schema_without_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test output_schema without a tool.

        The agent should compute the sum and return structured output directly.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Compute arithmetic and return the result in the structured format.",
            tools=[],
            output_schema=RESULT_SCHEMA,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 5 + 7?"},
        )

        assert result is not None
        # The result should be structured with a "result" field
        assert "sum" in result or "12" in str(result), f"Expected structured result with '12': {result}"

        # For Anthropic and Bedrock, verify structured_output tool was used
        if provider_config["name"] in ("anthropic", "bedrock"):
            messages = result.get("messages", [])
            has_structured_output_msg = any(
                msg.get("role") == "tool" and
                msg.get("content") == "Successfully ran structured_output tool"
                for msg in messages
            )
            assert has_structured_output_msg, (
                f"Expected 'Successfully ran structured_output tool' message for {provider_config['name']}: {messages}"
            )

        print(f"Output schema without tool result from {provider_config['name']}: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
