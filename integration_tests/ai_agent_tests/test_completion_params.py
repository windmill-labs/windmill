"""
Completion parameter tests for AI agents.

Tests that AI agents correctly handle temperature and max_completion_tokens:
- Default parameters (undefined)
- Low temperature (0.0 - deterministic)
- High temperature (0.9 - more random)
- Low max_completion_tokens (10 - short response)
- High max_completion_tokens (4096 - longer response allowed)
- Combined parameters
"""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow
from .providers import ALL_PROVIDERS, get_provider_ids


class TestCompletionParams:
    """Test AI agent temperature and max_completion_tokens parameters."""

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_default_params(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with default parameters (no temperature or max_completion_tokens).
        This serves as a baseline to ensure the agent works without these params.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            output_type="text",
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Answer with just the number."},
        )

        assert result is not None
        # Result should contain the answer
        result_str = str(result)
        assert "4" in result_str, f"Expected '4' in result: {result}"

        print(f"Default params result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_low_temperature(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with temperature=0.0 (deterministic output).
        Low temperature should produce more focused, consistent responses.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            output_type="text",
            temperature=0.0,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Answer with just the number."},
        )

        assert result is not None
        result_str = str(result)
        assert "4" in result_str, f"Expected '4' in result: {result}"

        print(f"Low temperature (0.0) result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_high_temperature(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with temperature=0.9 (more random output).
        High temperature should still produce valid responses.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            output_type="text",
            temperature=0.9,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Answer with just the number."},
        )

        assert result is not None
        # With high temperature, the model might be more creative but should still respond
        result_str = str(result)
        # We just verify we got a non-empty response
        assert len(result_str) > 0, f"Expected non-empty result: {result}"

        print(f"High temperature (0.9) result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_low_max_tokens(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with max_completion_tokens=10 (short response).
        The response should be truncated or very short.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant.",
            output_type="text",
            max_completion_tokens=10,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Explain the theory of relativity in detail."},
        )

        assert result is not None
        # The response should be truncated due to low max_tokens
        # We verify we got some response (even if truncated)
        result_str = str(result)
        assert len(result_str) > 0, f"Expected non-empty result: {result}"

        print(f"Low max_tokens (10) result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_high_max_tokens(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with max_completion_tokens=4096 (longer response allowed).
        The model should be able to produce longer responses if needed.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            output_type="text",
            max_completion_tokens=4096,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Answer with just the number."},
        )

        assert result is not None
        result_str = str(result)
        assert "4" in result_str, f"Expected '4' in result: {result}"

        print(f"High max_tokens (4096) result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_combined_params(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """
        Test with both temperature and max_completion_tokens set.
        Verifies that both parameters work together correctly.
        """
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant. Be concise.",
            output_type="text",
            temperature=0.5,
            max_completion_tokens=100,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "What is 2 + 2? Answer with just the number."},
        )

        assert result is not None
        result_str = str(result)
        assert "4" in result_str, f"Expected '4' in result: {result}"

        print(f"Combined params (temp=0.5, max_tokens=100) result from {provider_config['name']}: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
