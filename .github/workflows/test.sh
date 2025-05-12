#!/bin/bash

# --- Set your test prompt here ---
ISSUE="mcp do not have all the scripts i have"

PROMPT="I'm giving you an issue that needs to be fixed. Your role is to give me the files that are relevant to the issue. The issue is prepended with the word ISSUE.
ISSUE: $ISSUE. Give me all the files relevant to this issue. Your output should be a single json array that can be parsed with programatic json parsing, with the relevant files. Exemple of output: [\"file1.py\", \"file2.py\"]"
# ---------------------------------

echo "Using prompt: $PROMPT"
echo "Running probe-chat to find relevant files..."

# Run probe-chat, redirect stderr to stdout to capture potential errors too
# The tool might output conversational text before the JSON, handle this.
# Using a temporary file for intermediate output can sometimes be more robust
PROBE_OUTPUT_FILE=$(mktemp)
npx -y @buger/probe-chat@latest --model-name claude-3-7-sonnet-latest --message "$PROMPT" > "$PROBE_OUTPUT_FILE" 2>&1

echo "--- Probe-chat raw output ---"
cat "$PROBE_OUTPUT_FILE"
echo "-----------------------------"

# Extract the JSON array part. Handles potential leading/trailing text.
# Extracts content between the first '[' and the last ']' inclusive.
JSON_FILES=$(sed -n '/^\s*\[/,$p' "$PROBE_OUTPUT_FILE" | sed '/^\s*\]/q')

# Clean up temporary file
rm "$PROBE_OUTPUT_FILE"

echo "Extracted JSON block:"
JSON_FILES="[\"file1.py\", \"file2.py\"]"
echo "$JSON_FILES"

# Parse JSON, filter for non-empty strings, quote each for shell, join with spaces.
# Default to empty string on any error (e.g., invalid JSON)
FILES_LIST=$(echo "$JSON_FILES" | jq -e -r '[.[] | select(type == "string" and . != "" and . != null and (endswith("/") | not))] | map(@sh) | join(" ")' || echo "")

echo "Formatted files list for aider: $FILES_LIST"

if [ -z "$FILES_LIST" ]; then
  echo "No files identified by probe-chat. Skipping aider execution."
  exit 0
fi

# Construct and run the aider command
AIDER_CMD="aider \
  $FILES_LIST \
  --model anthropic/claude-3-7-sonnet-latest \
  --message \"$ISSUE\" \
  --yes \
  --no-check-update \
  --auto-commits \
  --no-analytics \
  --no-stream" # Add other flags as necessary

echo ""
echo "--- Running aider command ---"
echo "$AIDER_CMD"
echo "-----------------------------"

# Execute the command
eval "$AIDER_CMD"
