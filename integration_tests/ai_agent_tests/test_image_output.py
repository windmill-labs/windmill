"""Tests for AI agent image output_type functionality."""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow
from .providers import GOOGLE_AI, get_provider_ids


# Only OpenAI supports image generation via DALL-E
IMAGE_GENERATION_PROVIDERS = [GOOGLE_AI]


class TestImageOutput:
    """Test AI agent image generation with output_type='image'."""

    @pytest.mark.parametrize(
        "provider_config",
        IMAGE_GENERATION_PROVIDERS,
        ids=get_provider_ids(IMAGE_GENERATION_PROVIDERS),
    )
    def test_image_generation_returns_s3_object(
        self,
        client: AIAgentTestClient,
        setup_providers,
        setup_s3_storage,
        provider_config,
    ):
        """Test that output_type='image' generates an image and returns S3 object."""
        input_transform = provider_config["input_transform"]
        input_transform["value"]["model"] = "gemini-2.5-flash-image-preview"

        flow_value = create_ai_agent_flow(
            provider_input_transform=input_transform,
            system_prompt="You are an image generation assistant.",
            output_type="image",
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={
                "user_message": "Generate a small simple windmill",
            },
        )

        # Verify we got an S3 object back
        assert result is not None
        assert isinstance(result, dict), f"Expected dict, got {type(result)}"
        assert "s3" in result, f"Expected 's3' key in result: {result}"
        assert result["s3"].startswith("ai_images/"), f"S3 key should start with 'ai_images/': {result['s3']}"
        assert result["s3"].endswith(".png"), f"S3 key should end with '.png': {result['s3']}"
        print(f"Image generation result: {result}")
