"""
Bedrock auth mode tests for AI agents.

Verifies Bedrock works with the following resource auth modes:
- API key
- IAM access key + secret key (no session token)
- Environment credential fallback
- IAM access key + secret key + session token
"""

import os

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow
from .providers import BEDROCK_API_KEY, BEDROCK_ENV, BEDROCK_IAM, BEDROCK_IAM_SESSION


class TestBedrockAuthModes:
    """Test AI agent Bedrock auth modes."""

    @pytest.mark.usefixtures("setup_providers")
    def test_bedrock_api_key_resource(self, client: AIAgentTestClient):
        """Test Bedrock with API key resource auth."""
        if not os.environ.get("BEDROCK_API_KEY"):
            pytest.skip("BEDROCK_API_KEY not set")

        flow_value = create_ai_agent_flow(
            provider_input_transform=BEDROCK_API_KEY["input_transform"],
            system_prompt="You are a concise assistant.",
        )
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Reply with one short sentence."},
        )

        assert isinstance(result.get("output"), str), f"Expected string output, got: {result}"
        assert result.get("output", "").strip(), f"Expected non-empty output, got: {result}"

    @pytest.mark.usefixtures("setup_providers")
    def test_bedrock_iam_resource_without_session_token(self, client: AIAgentTestClient):
        """Test Bedrock with IAM access key + secret key auth."""
        if not (os.environ.get("AWS_ACCESS_KEY_ID") and os.environ.get("AWS_SECRET_ACCESS_KEY")):
            pytest.skip("AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY required")

        flow_value = create_ai_agent_flow(
            provider_input_transform=BEDROCK_IAM["input_transform"],
            system_prompt="You are a concise assistant.",
        )
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Reply with one short sentence."},
        )

        assert isinstance(result.get("output"), str), f"Expected string output, got: {result}"
        assert result.get("output", "").strip(), f"Expected non-empty output, got: {result}"

    @pytest.mark.usefixtures("setup_providers")
    def test_bedrock_environment_resource(self, client: AIAgentTestClient):
        """Test Bedrock with environment credential fallback."""
        if not (
            os.environ.get("AWS_ACCESS_KEY_ID")
            and os.environ.get("AWS_SECRET_ACCESS_KEY")
        ):
            pytest.skip("AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY required for env fallback")

        flow_value = create_ai_agent_flow(
            provider_input_transform=BEDROCK_ENV["input_transform"],
            system_prompt="You are a concise assistant.",
        )
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Reply with one short sentence."},
        )

        assert isinstance(result.get("output"), str), f"Expected string output, got: {result}"
        assert result.get("output", "").strip(), f"Expected non-empty output, got: {result}"

    @pytest.mark.usefixtures("setup_providers")
    def test_bedrock_iam_resource_with_session_token(self, client: AIAgentTestClient):
        """Test Bedrock with IAM access key + secret key + session token auth."""
        if not (
            os.environ.get("AWS_ACCESS_KEY_ID")
            and os.environ.get("AWS_SECRET_ACCESS_KEY")
            and os.environ.get("AWS_SESSION_TOKEN")
        ):
            pytest.skip(
                "AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, and AWS_SESSION_TOKEN required"
            )

        flow_value = create_ai_agent_flow(
            provider_input_transform=BEDROCK_IAM_SESSION["input_transform"],
            system_prompt="You are a concise assistant.",
        )
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Reply with one short sentence."},
        )

        assert isinstance(result.get("output"), str), f"Expected string output, got: {result}"
        assert result.get("output", "").strip(), f"Expected non-empty output, got: {result}"
