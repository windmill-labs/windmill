---
name: write-script-powershell
description: Write PowerShell scripts with param() function syntax.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# Windmill Script Writing Guide

## General Principles

- Scripts must export a main function (do not call it)
- Libraries are installed automatically - do not show installation instructions
- Credentials and configuration are stored in resources and passed as parameters
- The windmill client (`wmill`) provides APIs for interacting with the platform

## Function Naming

- Main function: `main` (or `preprocessor` for preprocessor scripts)
- Must be async for TypeScript variants

## Return Values

- Scripts can return any JSON-serializable value
- Return values become available to subsequent flow steps via `results.step_id`

## Preprocessor Scripts

Preprocessor scripts process raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, Postgres, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.

The returned object determines the parameter values passed to the flow.
e.g., `{ b: 1, a: 2 }` calls the flow with `a = 2` and `b = 1`, assuming the flow has two inputs called `a` and `b`.

The preprocessor receives a single parameter called `event`.


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
