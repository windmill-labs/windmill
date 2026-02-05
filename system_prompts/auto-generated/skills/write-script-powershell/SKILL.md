---
name: write-script-powershell
description: MUST use when writing PowerShell scripts.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

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
