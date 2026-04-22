"""Tests for AI agent user_attachments (PDF) functionality with S3 storage.

Prerequisites:
- MinIO running on localhost:9000 with bucket 'wmill'
- Test files uploaded to MinIO (test_images/test_image.webp, test_documents/test_document.pdf)
- S3 resource and storage configured in the integration-tests workspace
"""

import pytest

from .conftest import AIAgentTestClient, create_ai_agent_flow, TEST_IMAGE_S3_KEY
from .providers import VISION_PROVIDERS, get_provider_ids

TEST_PDF_S3_KEY = "test_documents/test_document.pdf"


class TestUserAttachments:
    """Test AI agent with PDF attachments from S3 storage."""

    @pytest.mark.parametrize(
        "provider_config",
        VISION_PROVIDERS,
        ids=get_provider_ids(VISION_PROVIDERS),
    )
    def test_pdf_analysis(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test that AI can analyze a PDF uploaded to S3."""
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant that reads documents. Be concise.",
            include_user_images=True,
        )

        # Run the flow with the PDF (test_document.pdf contains "Hello PDF World")
        result = client.run_preview_flow(
            flow_value=flow_value,
            args={
                "user_message": "What text does this PDF document contain? Reply with just the text.",
                "user_images": [
                    {
                        "s3": TEST_PDF_S3_KEY,
                        "storage": None,
                        "filename": "test_document.pdf",
                    }
                ],
            },
        )

        assert result is not None
        assert isinstance(result, (dict, str))
        result_text = str(result).lower()
        assert "hello" in result_text or "pdf" in result_text, (
            f"Expected AI to read PDF content containing 'Hello PDF World', "
            f"got: {result}"
        )
        print(f"PDF analysis result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        VISION_PROVIDERS,
        ids=get_provider_ids(VISION_PROVIDERS),
    )
    def test_backward_compat_user_images(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test that the old user_images field name still works for images."""
        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="You are a helpful assistant that describes images. Be concise.",
            include_user_images=True,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={
                "user_message": "Describe what you see in this image in one sentence.",
                "user_images": [
                    {
                        "s3": TEST_IMAGE_S3_KEY,
                        "storage": None,
                        "filename": "test_image.webp",
                    }
                ],
            },
        )

        assert result is not None
        assert isinstance(result, (dict, str))
        print(f"Backward compat image result from {provider_config['name']}: {result}")
