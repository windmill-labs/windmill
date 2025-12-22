"""
Streaming tests for AI agents.

Tests that AI agents correctly handle streaming responses:
- Streaming enabled returns wm_stream with valid events
- Streaming with tools includes tool-related events
- Streaming disabled does not return wm_stream
"""

import json

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


def parse_streaming_events(wm_stream: str) -> list[dict]:
    """Parse newline-delimited JSON events from wm_stream."""
    events = []
    for line in wm_stream.strip().split("\n"):
        if line:
            events.append(json.loads(line))
    return events


class TestStreaming:
    """Test AI agent streaming functionality."""

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_streaming_enabled(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that streaming enabled returns wm_stream with valid events.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Answer concisely.",
            streaming=True,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Just say the number."},
        )

        assert result is not None
        assert "wm_stream" in result, f"Expected 'wm_stream' in result: {result}"

        wm_stream = result["wm_stream"]
        assert wm_stream is not None and wm_stream != "", "wm_stream should not be empty"

        # Parse and validate events
        events = parse_streaming_events(wm_stream)
        assert len(events) > 0, "Expected at least one streaming event"

        # Check that events have valid types (snake_case format)
        valid_types = {"token_delta", "tool_call", "tool_call_arguments", "tool_execution", "tool_result"}
        for event in events:
            assert "type" in event, f"Event missing 'type' field: {event}"
            assert event["type"] in valid_types, f"Invalid event type: {event['type']}"

        # For a simple response without tools, we expect token_delta events
        token_events = [e for e in events if e["type"] == "token_delta"]
        assert len(token_events) > 0, "Expected at least one TokenDelta event"

        print(f"Streaming result from {provider_config['name']}: {len(events)} events")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_streaming_with_tool(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that streaming with a tool includes tool-related events.
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
            streaming=True,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 5 + 7? Use the add_numbers tool."},
        )

        assert result is not None
        assert "wm_stream" in result, f"Expected 'wm_stream' in result: {result}"

        wm_stream = result["wm_stream"]
        assert wm_stream is not None and wm_stream != "", "wm_stream should not be empty"

        # Parse and validate events
        events = parse_streaming_events(wm_stream)
        assert len(events) > 0, "Expected at least one streaming event"

        # With a tool call, we expect tool-related events
        event_types = {e["type"] for e in events}

        # Should have at least some of the tool events (snake_case format)
        tool_event_types = {"tool_call", "tool_call_arguments", "tool_execution", "tool_result"}
        has_tool_events = bool(event_types & tool_event_types)
        assert has_tool_events, f"Expected tool events, got: {event_types}"

        print(f"Streaming with tool result from {provider_config['name']}: {event_types}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_streaming_disabled(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that streaming disabled does not return wm_stream.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Answer concisely.",
            streaming=False,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Just say the number."},
        )

        assert result is not None

        # wm_stream should not be present or should be empty/null
        wm_stream = result.get("wm_stream")
        assert wm_stream is None, f"Expected no wm_stream when disabled"

        print(f"Non-streaming result from {provider_config['name']}: no wm_stream (as expected)")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
