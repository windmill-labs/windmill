"""Tests for AI agent user_images functionality with S3 storage."""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow, TEST_IMAGE_S3_KEY
from .providers import VISION_PROVIDERS, get_provider_ids


class TestUserImages:
    """Test AI agent with user images from S3 storage."""

    @pytest.mark.parametrize(
        "provider_config",
        VISION_PROVIDERS,
        ids=get_provider_ids(VISION_PROVIDERS),
    )
    def test_user_image_analysis(
        self,
        client: AIAgentTestClient,
        setup_providers,
        setup_s3_storage,
        provider_config,
    ):
        """Test that AI can analyze an image uploaded to S3."""
        # Create flow with user_images support
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant that describes images. Be concise.",
            include_user_images=True,
        )

        # Run the flow with the image (already uploaded by setup_s3_storage fixture)
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={
                "user_message": "Describe what you see in this image in one sentence.",
                "user_images": [
                    {
                        "s3": TEST_IMAGE_S3_KEY,
                        "storage": None,
                        "filename": "test_image.png",
                    }
                ],
            },
        )

        # Verify we got a response (the AI successfully processed the image)
        assert result is not None
        assert isinstance(result, (dict, str))
        print(f"User image analysis result from {provider_config['name']}: {result}")
