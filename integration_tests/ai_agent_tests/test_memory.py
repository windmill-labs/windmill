"""
Memory tests for AI agents.

Tests that AI agents correctly handle conversation memory/history.
"""

import os
import pytest
import uuid
from typing import Any

from .conftest import AIAgentTestClient, create_ai_agent_flow
from .providers import (
    ALL_PROVIDERS,
    BEDROCK_API_KEY,
    BEDROCK_ENV,
    BEDROCK_IAM,
    BEDROCK_IAM_SESSION,
    get_provider_ids,
)


class TestMemory:
    """Test AI agent memory functionality."""

    @staticmethod
    def _pick_bedrock_provider() -> dict[str, Any]:
        if os.environ.get("BEDROCK_API_KEY"):
            return BEDROCK_API_KEY

        if (
            os.environ.get("BEDROCK_IAM_ACCESS_KEY_ID")
            and os.environ.get("BEDROCK_IAM_SECRET_ACCESS_KEY")
        ):
            return BEDROCK_IAM

        if (
            os.environ.get("BEDROCK_SESSION_ACCESS_KEY_ID")
            and os.environ.get("BEDROCK_SESSION_SECRET_ACCESS_KEY")
            and os.environ.get("BEDROCK_SESSION_TOKEN")
        ):
            return BEDROCK_IAM_SESSION

        aws_access_key_id = os.environ.get("AWS_ACCESS_KEY_ID")
        aws_secret_access_key = os.environ.get("AWS_SECRET_ACCESS_KEY")
        aws_session_token = os.environ.get("AWS_SESSION_TOKEN")

        if aws_access_key_id and aws_secret_access_key:
            if aws_access_key_id.startswith("ASIA") and not aws_session_token:
                pytest.skip(
                    "AWS_SESSION_TOKEN required for Bedrock env fallback when AWS_ACCESS_KEY_ID is temporary (ASIA...)"
                )
            return BEDROCK_ENV

        pytest.skip("No Bedrock credentials available for the memory regression test")

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

    def test_memory_keeps_single_system_prompt_across_turns(
        self,
        client: AIAgentTestClient,
        setup_providers,
    ):
        """
        Test that auto memory persists conversation turns without persisting
        duplicate system prompts across repeated runs.
        """
        provider_config = self._pick_bedrock_provider()

        system_prompt = "You are a helpful assistant. Keep responses short."
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt=system_prompt,
            context_length=6,
        )

        memory_id = str(uuid.uuid4())
        turns = [
            "Turn one: say hi.",
            "Turn two: say hi again.",
            "Turn three: say hi one more time.",
        ]

        result = None
        for turn in turns:
            result = client.run_preview_flow(
                flow_value=flow_value,
                args={"user_message": turn},
                memory_id=memory_id,
            )
            assert result is not None
            assert "error" not in result, f"Turn failed for {provider_config['name']}: {result}"

        assert result is not None
        messages = result.get("messages", [])
        system_messages = [msg for msg in messages if msg.get("role") == "system"]

        assert len(system_messages) == 1, (
            f"Expected exactly one system message for {provider_config['name']}, got: {messages}"
        )
        assert system_messages[0].get("content") == system_prompt

        user_messages = [
            msg.get("content")
            for msg in messages
            if msg.get("role") == "user"
        ]
        assert user_messages == turns, (
            f"Expected all prior user turns to remain in memory for {provider_config['name']}: {messages}"
        )

        print(f"System prompt dedupe test passed for {provider_config['name']}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
