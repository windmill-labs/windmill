"""
Memory tests for AI agents.

Tests that AI agents correctly handle conversation memory/history.
"""

import pytest
import uuid
from typing import Any

from .conftest import AIAgentTestClient, create_ai_agent_flow
from .providers import ALL_PROVIDERS, get_provider_ids


class TestMemory:
    """Test AI agent memory functionality."""

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_memory_two_messages(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test that sending two messages with the same memory_id works correctly.
        The second message should include the conversation history from the first.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            context_length=2,
        )

        # Use a shared memory_id for both messages
        memory_id = str(uuid.uuid4())

        # First message
        result1 = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "My name is Alice. Remember this."},
            memory_id=memory_id,
        )

        assert result1 is not None
        assert "error" not in result1, f"First message failed: {result1}"

        # Second message - should remember the first
        result2 = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is my name?"},
            memory_id=memory_id,
        )

        assert result2 is not None
        assert "error" not in result2, f"Second message failed: {result2}"

        # The response should mention Alice
        response = str(result2.get("output", "")).lower()
        assert "alice" in response, f"Expected 'Alice' in response: {result2}"

        print(f"Memory test passed for {provider_config['name']}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
