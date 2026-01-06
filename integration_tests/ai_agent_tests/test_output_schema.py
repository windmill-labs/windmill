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


class TestSchemaVariations:
    """
    Test various schema features across all providers.

    These tests verify that different JSON Schema features are correctly
    processed by make_strict() and accepted by providers.
    """

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_nested_objects_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test deeply nested object structure."""
        schema = {
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"type": "integer"}
                    }
                }
            },
            "required": ["user"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Extract user info. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "User John is 25 years old"},
        )

        assert result is not None
        assert "user" in result or "John" in str(result)
        print(f"Nested objects result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_array_of_objects_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test array with object items."""
        schema = {
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "integer"},
                            "label": {"type": "string"}
                        }
                    }
                }
            },
            "required": ["items"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Create a list of items. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Create 2 items: Apple (id 1), Banana (id 2)"},
        )

        assert result is not None
        assert "items" in result or "Apple" in str(result)
        print(f"Array of objects result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_enum_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test enum constraints."""
        schema = {
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["pending", "approved", "rejected"]
                }
            },
            "required": ["status"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Classify the request status. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "The request was accepted"},
        )

        assert result is not None
        status = result.get("status") if isinstance(result, dict) else None
        assert status in ["pending", "approved", "rejected"] or "approved" in str(result)
        print(f"Enum result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_optional_fields_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test that optional fields are handled correctly (made nullable)."""
        schema = {
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "nickname": {"type": "string"}  # Not in required - should be nullable
            },
            "required": ["name"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Extract name info. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "The person is called Alice"},
        )

        assert result is not None
        assert "name" in result or "Alice" in str(result)
        print(f"Optional fields result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_number_constraints_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test min/max constraints on numbers."""
        schema = {
            "type": "object",
            "properties": {
                "rating": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 5
                }
            },
            "required": ["rating"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Provide a rating from 1 to 5. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "This is excellent, rate it highly"},
        )

        assert result is not None
        rating = result.get("rating") if isinstance(result, dict) else None
        if rating is not None:
            assert 1 <= rating <= 5, f"Rating {rating} out of bounds"
        print(f"Number constraints result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_definitions_ref_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test $ref with definitions."""
        if provider_config["name"] == "google_ai":
            pytest.xfail("Google AI does not support $ref with definitions in output_schema")

        schema = {
            "type": "object",
            "properties": {
                "primary": {"$ref": "#/definitions/Color"},
                "secondary": {"$ref": "#/definitions/Color"}
            },
            "required": ["primary", "secondary"],
            "definitions": {
                "Color": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "hex": {"type": "string"}
                    },
                    "required": ["name", "hex"]
                }
            }
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Provide color information. Return structured data with primary and secondary colors.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "Primary color is red (#FF0000), secondary is blue (#0000FF)"},
        )

        assert result is not None
        assert "primary" in result or "red" in str(result).lower()
        print(f"Definitions/ref result from {provider_config['name']}: {result}")

    @pytest.mark.parametrize(
        "provider_config",
        ALL_PROVIDERS,
        ids=get_provider_ids(ALL_PROVIDERS),
    )
    def test_anyof_schema(
        self,
        client: AIAgentTestClient,
        setup_providers,
        provider_config,
    ):
        """Test anyOf for union types."""
        schema = {
            "type": "object",
            "properties": {
                "value": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "number"}
                    ]
                }
            },
            "required": ["value"]
        }

        flow_value = create_ai_agent_flow(
            provider_input_transform=provider_config["input_transform"],
            system_prompt="Extract the value. Return structured data.",
            tools=[],
            output_schema=schema,
        )

        result = client.run_preview_flow(
            flow_value=flow_value,
            args={"user_message": "The answer is 42"},
        )

        assert result is not None
        assert "value" in result or "42" in str(result)
        print(f"anyOf result from {provider_config['name']}: {result}")


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
