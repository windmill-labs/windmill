# AI Agents Integration Tests

Integration tests for Windmill AI agents using the `preview_flow` endpoint.

## Quick Start

1. Create and activate a virtual environment:
   ```bash
   cd integration_tests/ai_agent_tests
   python -m venv .venv
   source .venv/bin/activate  # On Windows: .venv\Scripts\activate
   ```

2. Install dependencies:
   ```bash
   pip install -r requirements.txt
   ```

3. Create `.env` file with your API keys:
   ```
   OPENAI_API_KEY=sk-...
   ANTHROPIC_API_KEY=sk-ant-...
   GOOGLE_AI_API_KEY=...
   OPENROUTER_API_KEY=sk-or-...
   BEDROCK_API_KEY=...
   BEDROCK_IAM_ACCESS_KEY_ID=...
   BEDROCK_IAM_SECRET_ACCESS_KEY=...
   BEDROCK_SESSION_ACCESS_KEY_ID=...
   BEDROCK_SESSION_SECRET_ACCESS_KEY=...
   BEDROCK_SESSION_TOKEN=...  # optional, only for session-token Bedrock test
   AWS_ACCESS_KEY_ID=...      # optional, for env-fallback Bedrock test
   AWS_SECRET_ACCESS_KEY=...  # optional, for env-fallback Bedrock test
   AWS_SESSION_TOKEN=...      # optional, required for env-fallback if AWS_ACCESS_KEY_ID is ASIA...
   BEDROCK_REGION=us-east-1  # optional
   ```

4. Run a single test to verify:
   ```bash
   pytest test_basic_completion.py::TestOpenAI::test_openai_completion -v -s
   ```

## Prerequisites

1. A running Windmill instance (default: `http://localhost:8000`)
2. Python 3.10+
3. API keys for the providers you want to test

## How It Works

### Provider Setup

The `setup_providers` fixture automatically creates Windmill variables and resources from your environment variables:

| Provider | Variable Path | Resource Path | Resource Type |
|----------|--------------|---------------|---------------|
| OpenAI | `u/admin/openai_api_key` | `u/admin/openai` | openai |
| Anthropic | `u/admin/anthropic_api_key` | `u/admin/anthropic` | anthropic |
| Google AI | `u/admin/google_ai_api_key` | `u/admin/googleai` | googleai |
| OpenRouter | `u/admin/openrouter_api_key` | `u/admin/openrouter` | openrouter |
| Bedrock (api key) | `u/admin/bedrock_api_key` | `u/admin/bedrock` | aws_bedrock |
| Bedrock (IAM) | `u/admin/bedrock_iam_access_key_id`, `u/admin/bedrock_iam_secret_access_key` | `u/admin/bedrock_iam` | aws_bedrock |
| Bedrock (IAM + session) | `u/admin/bedrock_session_access_key_id`, `u/admin/bedrock_session_secret_access_key`, `u/admin/bedrock_session_token` | `u/admin/bedrock_iam_session` | aws_bedrock |
| Bedrock (environment fallback) | none (uses worker/api AWS env) | `u/admin/bedrock_env` | aws_bedrock |

### Tools

Tools use **rawscript** with inline content instead of creating actual scripts. This means:
- No script creation/deployment needed
- Scripts are embedded directly in the flow definition
- Faster test execution

## Running Tests

All commands assume you're in the `integration_tests/ai_agent_tests` directory with the venv activated.

### Run all AI agent tests

```bash
pytest . -v -s
```

### Run only basic completion tests

```bash
pytest test_basic_completion.py -v -s
```

### Run only tool calling tests

```bash
pytest test_tool_calling.py -v -s
```

### Run tests for a specific provider

```bash
# Only Anthropic tests
pytest . -v -s -k "anthropic"

# Only OpenAI tests
pytest . -v -s -k "openai"
```

### Run a single test

```bash
# OpenAI basic completion
pytest test_basic_completion.py::TestOpenAI::test_openai_completion -v -s

# Anthropic basic completion
pytest test_basic_completion.py::TestAnthropic::test_anthropic_completion -v -s

# Parametrized test for one provider
pytest test_basic_completion.py::TestBasicCompletion::test_simple_prompt[openai] -v -s
```

## Test Structure

- `conftest.py` - Test fixtures and utilities
  - `AIAgentTestClient` - HTTP client for preview_flow
  - `create_ai_agent_flow()` - Creates AI agent flow definitions
  - `create_rawscript_tool()` - Creates inline script tools
  - `setup_providers` - Sets up variables and resources
- `providers.py` - Provider configurations (OpenAI, Anthropic, Google AI, Bedrock, OpenRouter)
- `test_basic_completion.py` - Basic AI completion tests
- `test_tool_calling.py` - Tool calling tests with inline scripts

## Troubleshooting

### Tests fail with authentication error

Make sure Windmill is running and you can login with the default credentials:
- Email: `admin@windmill.dev`
- Password: `changeme`

### Tests skip due to missing API keys

Ensure the environment variables are set correctly:

```bash
echo $OPENAI_API_KEY
```

### Resource creation fails

If resources already exist with different values, you may need to delete them manually via the Windmill UI or API before running tests again.
