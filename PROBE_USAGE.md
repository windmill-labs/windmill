# Probe

[![npm version](https://img.shields.io/npm/v/@buger/probe)](https://www.npmjs.com/package/@buger/probe)

Probe is a powerful, AI-augmented code search and extraction tool. It combines fast, tree-sitter–powered queries with optional LLM integration to help you find, extract, and understand code across any codebase.

---

## 📦 Installation

The easiest way to install Probe is via npm:

```bash
npm install -g @buger/probe@latest
```

Or using curl (for macOS and Linux):

```bash
curl -fsSL https://raw.githubusercontent.com/buger/probe/main/install.sh | bash
```

## 🔍 Basic Search Example

Search for code containing specific phrases in the current directory:

```bash
probe search "parser" ./
```

This will search for the terms "parser" in your codebase and return the most relevant code blocks.

## 🔄 Advanced Search (with Token Limiting)

Limit the total tokens for AI tools with context window constraints:

```bash
probe search "prompt injection" ./ --max-tokens 10000
```

## 💾 Session-Based Caching

Use session IDs to avoid seeing the same code blocks multiple times in related searches:

```bash
# First search - generates a session ID
probe search "authentication" --session ""
# Session: a1b2 (example output)

# Subsequent searches - reuse the session ID
probe search "login" --session "a1b2"
# Will skip code blocks already shown in the previous search
```

## 🔎 Elastic Search Queries

Use advanced query syntax for more powerful searches:

```bash
# Use AND operator for terms that must appear together
probe search "error AND handling" ./

# Use OR operator for alternative terms
probe search "login OR authentication OR auth" ./src

# Group terms with parentheses for complex queries
probe search "(error OR exception) AND (handle OR process)" ./

# Use wildcards for partial matching
probe search "auth* connect*" ./

# Exclude terms with NOT operator
probe search "database NOT sqlite" ./
```

## 📋 Extract Code Blocks

Extract a specific function or code block containing a specific line:

```bash
probe extract src/main.rs:42
```

This uses tree-sitter to find the closest suitable parent node (function, struct, class, etc.) for that line.

You can even pipe failing test output:

```bash
go test | probe extract
```

Extract code with LLM prompt and instructions for AI integration:

```bash
# Extract with engineer prompt template
probe extract src/auth.rs#authenticate --prompt engineer --instructions "Explain this authentication function"

# Extract with architect prompt template
probe extract src/api.js --prompt architect --instructions "Analyze this API module"
```

## 🔍 Query Code Structures

Find specific code structures using tree-sitter patterns:

```bash
# Find JavaScript functions
probe query "function $NAME($$$PARAMS) $$$BODY" ./src --language javascript

# Find Python functions
probe query "def $NAME($$$PARAMS): $$$BODY" ./src --language python

# Find Go structs
probe query "type $NAME struct { $$$FIELDS }" ./src --language go
```

## 💬 Interactive AI Chat

Use the built-in AI assistant with web interface:

```bash
# Run directly with npx (no installation needed)
npx -y @buger/probe-chat@latest --web
npx -y @buger/probe-chat@latest

# Set your API key first
export ANTHROPIC_API_KEY=your_api_key
# Or for OpenAI
# export OPENAI_API_KEY=your_api_key
# Or for Gemini
# export GOOGLE_API_KEY=your_api_key

# Specify a directory to search (optional)
npx -y @buger/probe-chat@latest /path/to/your/project
```

Example questions you might ask:

- "How does the workers pick up jobs?"
- "How does the mcp functionality work?"
- "What are the main components of the windmill backend?"

## 🔌 MCP Server Integration

Integrate with any AI editor by adding this to your MCP configuration:

```json
{
  "mcpServers": {
    "memory": {
      "command": "npx",
      "args": ["-y", "@buger/probe-mcp"]
    }
  }
}
```

## 📖 Official Documentation

https://probeai.dev/features
