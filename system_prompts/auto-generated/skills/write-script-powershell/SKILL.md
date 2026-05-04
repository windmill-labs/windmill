---
name: write-script-powershell
description: MUST use when writing PowerShell scripts.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — generate `.script.yaml` and `.lock` files for the script you modified.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill sync push` and `wmill generate-metadata` modify workspace state or local files — only run these when the user explicitly asks; otherwise tell them which to run.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

# PowerShell

## Structure

Arguments are obtained by calling the `param` function on the first line:

```powershell
param($Name, $Count = 0, [int]$Age)

# Your code here
Write-Output "Processing $Name, count: $Count, age: $Age"

# Return object
@{
    name = $Name
    count = $Count
    age = $Age
}
```

## Parameter Types

You can specify types for parameters:

```powershell
param(
    [string]$Name,
    [int]$Count = 0,
    [bool]$Enabled = $true,
    [array]$Items
)

@{
    name = $Name
    count = $Count
    enabled = $Enabled
    items = $Items
}
```

## Return Values

Return values by outputting them at the end of the script:

```powershell
param($Input)

$result = @{
    processed = $true
    data = $Input
    timestamp = Get-Date -Format "o"
}

$result
```
