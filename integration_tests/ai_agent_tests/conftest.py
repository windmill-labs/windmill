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

# Constants for S3/user_images tests
TEST_IMAGE_PATH = Path(__file__).parent / "test_image.webp"
TEST_IMAGE_S3_KEY = "test_images/test_image.webp"


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

    def create_resource(self, path: str, resource_type: str, value: dict[str, Any]):
        """Create a resource that can reference variables."""
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

    def script_exists(self, path: str) -> bool:
        """Check if a script exists at the given path."""
        response = self._client.get(
            f"/api/w/{self.workspace}/scripts/exists/p/{path}",
        )
        return response.status_code == 200 and response.content.decode() == "true"

    def create_script(
        self,
        path: str,
        content: str,
        language: str = "bun",
        summary: str = "",
        description: str = "",
        schema: dict[str, Any] | None = None,
    ):
        """Create a script in the workspace."""
        # Check if script already exists
        if self.script_exists(path):
            return

        payload = {
            "path": path,
            "summary": summary,
            "description": description,
            "content": content,
            "schema": schema or {"type": "object", "properties": {}, "required": []},
            "is_template": False,
            "language": language,
            "kind": "script",
        }

        response = self._client.post(
            f"/api/w/{self.workspace}/scripts/create",
            json=payload,
        )
        if response.status_code // 100 != 2:
            error = response.content.decode()
            # Ignore if script already exists
            if "already exists" not in error.lower():
                raise Exception(f"Failed to create script: {error}")

    def upload_s3_file(self, s3_key: str, file_content: bytes, content_type: str = "image/png") -> dict:
        """Upload a file to S3 storage via Windmill API."""
        response = self._client.post(
            f"/api/w/{self.workspace}/job_helpers/upload_s3_file",
            params={"file_key": s3_key},
            content=file_content,
            headers={"Content-Type": content_type},
        )
        response.raise_for_status()
        return response.json()

    def delete_s3_file(self, s3_key: str, storage: str | None = None) -> dict:
        """Delete a file from S3 storage via Windmill API."""
        params = {"file_key": s3_key}
        if storage:
            params["storage"] = storage

        response = self._client.delete(
            f"/api/w/{self.workspace}/job_helpers/delete_s3_file",
            params=params,
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to delete S3 file: {response.content.decode()}")
        return response.json()

    def configure_s3_storage(self, s3_resource_path: str):
        """Configure workspace large file storage with S3."""
        response = self._client.post(
            f"/api/w/{self.workspace}/workspaces/edit_large_file_storage_config",
            json={
                "large_file_storage": {
                    "type": "S3Storage",
                    "s3_resource_path": s3_resource_path,
                    "public_resource": False,
                    "secondary_storage": {},
                    "advanced_permissions": [
                        {"allow": "read,write,delete", "pattern": "windmill_uploads/*"},
                        {"allow": "read,write,delete,list", "pattern": "u/{username}/**/*"},
                        {"allow": "read,write,delete,list", "pattern": "g/{group}/**/*"},
                        {"allow": "read,write,delete,list", "pattern": "f/{folder_write}/**/*"},
                        {"allow": "read,list", "pattern": "f/{folder_read}/**/*"},
                        {"allow": "", "pattern": "**/*"},
                    ],
                }
            },
        )
        if response.status_code // 100 != 2:
            raise Exception(f"Failed to configure S3 storage: {response.content.decode()}")


def create_ai_agent_flow(
    provider_input_transform: dict[str, Any],
    system_prompt: str = "You are a helpful assistant. Be concise.",
    tools: list[dict[str, Any]] | None = None,
    output_schema: dict[str, Any] | None = None,
    streaming: bool | None = None,
    include_user_images: bool = False,
    output_type: str | None = None,
    temperature: float | None = None,
    max_completion_tokens: int | None = None,
    context_length: int | None = None,
) -> dict[str, Any]:
    """
    Create a FlowValue for an AI agent.

    Args:
        provider_input_transform: The input transform for the provider field
        system_prompt: System prompt for the AI agent
        tools: Optional list of tool definitions
        output_schema: Optional JSON schema for structured output
        streaming: Optional flag to enable streaming responses
        include_user_images: If True, adds user_images input from flow_input
        output_type: Optional output type ("text" or "image")
        temperature: Optional temperature for sampling (0.0-2.0)
        max_completion_tokens: Optional maximum tokens for completion

    Returns:
        A FlowValue dictionary ready to be sent to preview_flow
    """
    input_transforms = {
        "provider": provider_input_transform,
        "system_prompt": {"type": "static", "value": system_prompt},
        "user_message": {"type": "javascript", "expr": "flow_input.user_message"},
    }

    # Add output_schema if provided
    if output_schema is not None:
        input_transforms["output_schema"] = {"type": "static", "value": output_schema}

    # Add streaming if provided
    if streaming is not None:
        input_transforms["streaming"] = {"type": "static", "value": streaming}

    # Add user_images if enabled
    if include_user_images:
        input_transforms["user_images"] = {"type": "javascript", "expr": "flow_input.user_images"}

    # Add output_type if provided
    if output_type is not None:
        input_transforms["output_type"] = {"type": "static", "value": output_type}

    # Add temperature if provided
    if temperature is not None:
        input_transforms["temperature"] = {"type": "static", "value": temperature}

    # Add max_completion_tokens if provided
    if max_completion_tokens is not None:
        input_transforms["max_completion_tokens"] = {"type": "static", "value": max_completion_tokens}

    # Add context_length if provided
    if context_length is not None:
        input_transforms["memory"] = {"type": "static", "value": {
            "kind": "auto",
            "context_length": context_length,
        }}

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


def create_websearch_tool() -> dict[str, Any]:
    """
    Create a websearch tool for AI agents.

    Returns:
        A websearch tool definition dictionary
    """
    return {
        "id": "websearch",
        "summary": "Web Search",
        "value": {
            "tool_type": "websearch",
        },
    }


def create_script_tool(
    tool_id: str,
    script_path: str,
    params: list[str],
) -> dict[str, Any]:
    """
    Create a tool that references an existing script in the workspace.

    Args:
        tool_id: Unique ID for the tool
        script_path: Path to the script in the workspace (e.g., "u/admin/sum_script")
        params: List of parameter names (each gets type: ai so the agent provides values)

    Returns:
        A tool definition dictionary
    """
    input_transforms = {param: {"type": "ai"} for param in params}

    return {
        "id": tool_id,
        "summary": tool_id,
        "value": {
            "tool_type": "flowmodule",
            "type": "script",
            "path": script_path,
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
    - BEDROCK_IAM_ACCESS_KEY_ID and BEDROCK_IAM_SECRET_ACCESS_KEY (optional, for IAM Bedrock tests)
    - BEDROCK_SESSION_ACCESS_KEY_ID, BEDROCK_SESSION_SECRET_ACCESS_KEY, BEDROCK_SESSION_TOKEN
      (optional, for IAM session Bedrock tests)
    - AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY (optional, for environment fallback tests)
    - AWS_SESSION_TOKEN (optional, if environment fallback uses temporary credentials)
    - BEDROCK_REGION (optional, defaults to us-east-1)
    """
    # OpenAI
    if os.environ.get("OPENAI_API_KEY"):
        client.create_variable("u/admin/openai_api_key", os.environ["OPENAI_API_KEY"])
        client.create_resource("u/admin/openai", "openai", {
            "api_key": "$var:u/admin/openai_api_key"
        })

    # Azure OpenAI
    if os.environ.get("AZURE_OPENAI_API_KEY") and os.environ.get("AZURE_OPENAI_BASE_URL"):
        client.create_variable("u/admin/azure_openai_api_key", os.environ["AZURE_OPENAI_API_KEY"])
        client.create_resource("u/admin/azure_openai", "azure_openai", {
            "apiKey": "$var:u/admin/azure_openai_api_key",
            "baseUrl": os.environ["AZURE_OPENAI_BASE_URL"],
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

    bedrock_region = os.environ.get("BEDROCK_REGION", "us-east-1")

    # Bedrock (using apiKey approach)
    if os.environ.get("BEDROCK_API_KEY"):
        client.create_variable("u/admin/bedrock_api_key", os.environ["BEDROCK_API_KEY"])
        client.create_resource("u/admin/bedrock", "aws_bedrock", {
            "apiKey": "$var:u/admin/bedrock_api_key",
            "region": bedrock_region
        })

    # Bedrock IAM credentials without session token
    if os.environ.get("BEDROCK_IAM_ACCESS_KEY_ID") and os.environ.get("BEDROCK_IAM_SECRET_ACCESS_KEY"):
        client.create_variable(
            "u/admin/bedrock_iam_access_key_id",
            os.environ["BEDROCK_IAM_ACCESS_KEY_ID"],
        )
        client.create_variable(
            "u/admin/bedrock_iam_secret_access_key",
            os.environ["BEDROCK_IAM_SECRET_ACCESS_KEY"],
        )
        client.create_resource("u/admin/bedrock_iam", "aws_bedrock", {
            "awsAccessKeyId": "$var:u/admin/bedrock_iam_access_key_id",
            "awsSecretAccessKey": "$var:u/admin/bedrock_iam_secret_access_key",
            "region": bedrock_region
        })

    # Bedrock IAM credentials with session token
    if (
        os.environ.get("BEDROCK_SESSION_ACCESS_KEY_ID")
        and os.environ.get("BEDROCK_SESSION_SECRET_ACCESS_KEY")
        and os.environ.get("BEDROCK_SESSION_TOKEN")
    ):
        client.create_variable(
            "u/admin/bedrock_session_access_key_id",
            os.environ["BEDROCK_SESSION_ACCESS_KEY_ID"],
        )
        client.create_variable(
            "u/admin/bedrock_session_secret_access_key",
            os.environ["BEDROCK_SESSION_SECRET_ACCESS_KEY"],
        )
        client.create_variable(
            "u/admin/bedrock_session_token",
            os.environ["BEDROCK_SESSION_TOKEN"],
        )
        client.create_resource("u/admin/bedrock_iam_session", "aws_bedrock", {
            "awsAccessKeyId": "$var:u/admin/bedrock_session_access_key_id",
            "awsSecretAccessKey": "$var:u/admin/bedrock_session_secret_access_key",
            "awsSessionToken": "$var:u/admin/bedrock_session_token",
            "region": bedrock_region
        })

    # Bedrock using environment credentials fallback
    # This resource intentionally omits explicit credentials.
    client.create_resource("u/admin/bedrock_env", "aws_bedrock", {
        "region": bedrock_region
    })

    # DeepWiki MCP resource (always created for MCP tool tests)
    client.create_resource("u/admin/deepwiki", "mcp", {
        "url": "https://mcp.deepwiki.com/mcp",
        "name": "deepwiki"
    })

    # Create sum script for workspace script tool tests
    client.create_script(
        path="u/admin/sum_script",
        content="""export function main(a: number, b: number): number {
    return a + b;
}
""",
        language="bun",
        summary="Sum two numbers",
        description="A simple script that adds two numbers together",
        schema={
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "a": {
                    "type": "number",
                    "description": "First number to add",
                },
                "b": {
                    "type": "number",
                    "description": "Second number to add",
                },
            },
            "required": ["a", "b"],
        },
    )

    yield


@pytest.fixture(scope="session")
def setup_s3_storage(client):
    """
    Set up MinIO S3 storage for user_images tests and upload test image.

    Expects the following environment variables to be set:
    - MINIO_ACCESS_KEY
    - MINIO_SECRET_KEY
    """
    access_key = os.environ.get("MINIO_ACCESS_KEY")
    secret_key = os.environ.get("MINIO_SECRET_KEY")

    if not access_key or not secret_key:
        pytest.skip("MINIO_ACCESS_KEY and MINIO_SECRET_KEY required for S3 tests")

    # 1. Create variable for secret key
    client.create_variable("u/admin/minio_secret_key", secret_key, is_secret=True)

    # 2. Create S3 resource for MinIO
    client.create_resource("u/admin/minio_s3", "s3", {
        "port": 9000,
        "bucket": "wmill",
        "region": "fr",
        "useSSL": False,
        "endPoint": "localhost",
        "accessKey": access_key,
        "pathStyle": True,
        "secretKey": "$var:u/admin/minio_secret_key",
    })

    # 3. Configure workspace large file storage
    client.configure_s3_storage("$res:u/admin/minio_s3")

    # 4. Upload test image to S3 right after setup
    if TEST_IMAGE_PATH.exists():
        with open(TEST_IMAGE_PATH, "rb") as f:
            image_content = f.read()
        client.upload_s3_file(TEST_IMAGE_S3_KEY, image_content)
    else:
        pytest.skip(f"Test image not found at {TEST_IMAGE_PATH}")

    yield TEST_IMAGE_S3_KEY
