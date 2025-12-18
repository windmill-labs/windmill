"""
Test fixtures and utilities for AI agent integration tests.
"""

import os
import uuid
from pathlib import Path
from typing import Any

import httpx
import pytest
from dotenv import load_dotenv

# Load .env file from the test directory
load_dotenv(Path(__file__).parent / ".env")


class AIAgentTestClient:
    """HTTP client for testing AI agents via the preview_flow endpoint."""

    def __init__(
        self,
        base_url: str = "http://localhost:8000",
        workspace: str = "integration-tests",
    ):
        self.base_url = base_url
        self.workspace = workspace
        self._token: str | None = None
        self._client: httpx.Client | None = None

    def _login(self) -> str:
        """Login and get auth token."""
        with httpx.Client(base_url=self.base_url) as client:
            response = client.post(
                "/api/auth/login",
                json={
                    "email": "admin@windmill.dev",
                    "password": "changeme",
                },
            )
            if response.status_code // 100 != 2:
                raise Exception(f"Login failed: {response.content.decode()}")
            return response.content.decode()

    def _ensure_workspace(self):
        """Create workspace if it doesn't exist."""
        exists = self._client.post(
            "/api/workspaces/exists",
            json={"id": self.workspace},
        )
        if exists.status_code // 100 == 2 and exists.content.decode() == "true":
            return
        response = self._client.post(
            "/api/workspaces/create",
            json={"id": self.workspace, "name": self.workspace},
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to create workspace: {response.content.decode()}")

    def connect(self):
        """Initialize connection and authenticate."""
        self._token = self._login()
        self._client = httpx.Client(
            base_url=self.base_url,
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {self._token}",
            },
            timeout=180.0,  # 3 minutes for AI operations
        )
        self._ensure_workspace()
        print(f"Connected to {self.base_url}, workspace: {self.workspace}")

    def close(self):
        """Close the HTTP client."""
        if self._client:
            self._client.post("/api/auth/logout")
            self._client.close()
            self._client = None

    def run_preview_flow(
        self,
        flow_value: dict[str, Any],
        args: dict[str, Any] | None = None,
        memory_id: str | None = None,
        timeout_secs: int | None = None,
    ) -> dict[str, Any]:
        """
        Run a preview flow and wait for the result.

        Args:
            flow_value: The FlowValue definition (modules, chat_input_enabled, etc.)
            args: Flow input arguments
            memory_id: Optional memory ID for chat conversations
            timeout_secs: Optional timeout in seconds

        Returns:
            The job result as a dictionary
        """
        if memory_id is None:
            memory_id = str(uuid.uuid4())

        payload = {
            "value": flow_value,
            "args": args or {},
        }

        params = {"memory_id": memory_id}
        if timeout_secs:
            params["timeout"] = timeout_secs

        response = self._client.post(
            f"/api/w/{self.workspace}/jobs/run_wait_result/preview_flow",
            params=params,
            json=payload,
        )

        if response.status_code // 100 != 2:
            raise Exception(
                f"Preview flow failed with status {response.status_code}: "
                f"{response.content.decode()}"
            )

        return response.json()

    def create_variable(self, path: str, value: str, is_secret: bool = True):
        """Create a variable (typically for API keys)."""
        print(f"Creating variable {path}")
        response = self._client.post(
            f"/api/w/{self.workspace}/variables/create",
            json={
                "path": path,
                "value": value,
                "description": "API key for AI agent testing",
                "is_secret": is_secret,
            },
        )
        if response.status_code // 100 != 2:
            error = response.content.decode()
            # Ignore if variable already exists
            if "already exists" not in error.lower():
                raise Exception(f"Failed to create variable: {error}")
            print(f"Variable {path} already exists, skipping")

    def create_resource(self, path: str, resource_type: str, value: dict[str, Any]):
        """Create a resource that can reference variables."""
        print(f"Creating resource {path} of type {resource_type}")
        response = self._client.post(
            f"/api/w/{self.workspace}/resources/create",
            json={
                "path": path,
                "resource_type": resource_type,
                "value": value,
                "description": "AI provider resource for testing",
            },
        )
        if response.status_code // 100 != 2:
            error = response.content.decode()
            # Ignore if resource already exists
            if "already exists" not in error.lower():
                raise Exception(f"Failed to create resource: {error}")
            print(f"Resource {path} already exists, skipping")


def create_ai_agent_flow(
    provider_input_transform: dict[str, Any],
    system_prompt: str = "You are a helpful assistant. Be concise.",
    tools: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """
    Create a FlowValue for an AI agent.

    Args:
        provider_input_transform: The input transform for the provider field
        system_prompt: System prompt for the AI agent
        tools: Optional list of tool definitions

    Returns:
        A FlowValue dictionary ready to be sent to preview_flow
    """
    input_transforms = {
        "provider": provider_input_transform,
        "system_prompt": {"type": "static", "value": system_prompt},
        "user_message": {"type": "javascript", "expr": "flow_input.user_message"},
    }

    module_value = {
        "type": "aiagent",
        "input_transforms": input_transforms,
        "tools": tools or [],
    }

    return {
        "modules": [
            {
                "id": "a",
                "value": module_value,
            }
        ],
        "chat_input_enabled": True,
    }


def create_rawscript_tool(
    tool_id: str,
    content: str,
    params: list[str],
    language: str = "bun"
) -> dict[str, Any]:
    """
    Create a tool using rawscript (inline script content).

    Args:
        tool_id: Unique ID for the tool
        content: The script content
        params: List of parameter names (each gets type: ai so the agent provides values)
        language: Script language (default: bun)

    Returns:
        A tool definition dictionary
    """
    input_transforms = {param: {"type": "ai"} for param in params}

    return {
        "id": tool_id,
        "summary": tool_id,
        "value": {
            "tool_type": "flowmodule",
            "type": "rawscript",
            "content": content,
            "language": language,
            "input_transforms": input_transforms,
        },
    }


@pytest.fixture(scope="session")
def client():
    """Create and return an AI agent test client."""
    client = AIAgentTestClient(
        base_url=os.environ.get("WINDMILL_URL", "http://localhost:8000"),
        workspace=os.environ.get("WINDMILL_WORKSPACE", "integration-tests"),
    )
    client.connect()
    yield client
    client.close()


@pytest.fixture(scope="session")
def setup_providers(client):
    """
    Set up API key variables and resources from environment variables.

    Expects the following environment variables to be set:
    - OPENAI_API_KEY
    - ANTHROPIC_API_KEY
    - GOOGLE_AI_API_KEY
    - OPENROUTER_API_KEY
    - BEDROCK_API_KEY (optional)
    """
    # OpenAI
    if os.environ.get("OPENAI_API_KEY"):
        client.create_variable("u/admin/openai_api_key", os.environ["OPENAI_API_KEY"])
        client.create_resource("u/admin/openai", "openai", {
            "api_key": "$var:u/admin/openai_api_key"
        })

    # Anthropic
    if os.environ.get("ANTHROPIC_API_KEY"):
        client.create_variable("u/admin/anthropic_api_key", os.environ["ANTHROPIC_API_KEY"])
        client.create_resource("u/admin/anthropic", "anthropic", {
            "apiKey": "$var:u/admin/anthropic_api_key"
        })

    # Google AI
    if os.environ.get("GOOGLE_AI_API_KEY"):
        client.create_variable("u/admin/google_ai_api_key", os.environ["GOOGLE_AI_API_KEY"])
        client.create_resource("u/admin/googleai", "googleai", {
            "api_key": "$var:u/admin/google_ai_api_key"
        })

    # OpenRouter
    if os.environ.get("OPENROUTER_API_KEY"):
        client.create_variable("u/admin/openrouter_api_key", os.environ["OPENROUTER_API_KEY"])
        client.create_resource("u/admin/openrouter", "openrouter", {
            "api_key": "$var:u/admin/openrouter_api_key"
        })

    # Bedrock (using apiKey approach)
    if os.environ.get("BEDROCK_API_KEY"):
        client.create_variable("u/admin/bedrock_api_key", os.environ["BEDROCK_API_KEY"])
        client.create_resource("u/admin/bedrock", "aws_bedrock", {
            "apiKey": "$var:u/admin/bedrock_api_key",
            "region": "us-east-1"
        })

    # DeepWiki MCP resource (always created for MCP tool tests)
    client.create_resource("u/admin/deepwiki", "mcp", {
        "url": "https://mcp.deepwiki.com/mcp",
        "name": "deepwiki"
    })

    yield
