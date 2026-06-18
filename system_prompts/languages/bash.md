# Bash

## Structure

Do not include `#!/bin/bash`. Arguments are obtained as positional parameters:

```bash
# Get arguments
var1="$1"
var2="$2"

echo "Processing $var1 and $var2"

# Return JSON by echoing to stdout
echo "{\"result\": \"$var1\", \"count\": $var2}"
```

**Important:**
- Do not include shebang (`#!/bin/bash`)
- Arguments are always strings
- Access with `$1`, `$2`, etc.

## Output

The script output is captured as the result. For structured data, output valid JSON:

```bash
name="$1"
count="$2"

# Output JSON result
cat << EOF
{
  "name": "$name",
  "count": $count,
  "timestamp": "$(date -Iseconds)"
}
EOF
```

## Environment Variables

Environment variables set in Windmill are available:

```bash
# Access environment variable
echo "Workspace: $WM_WORKSPACE"
echo "Job ID: $WM_JOB_ID"
```
