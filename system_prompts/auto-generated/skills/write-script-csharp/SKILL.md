---
name: write-script-csharp
description: Write C# scripts with NuGet #r directive for dependencies.
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


# C#

The script must contain a public static `Main` method inside a class:

```csharp
public class Script
{
    public static object Main(string name, int count)
    {
        return new { Name = name, Count = count };
    }
}
```

**Important:**
- Class name is irrelevant
- Method must be `public static`
- Return type can be `object` or specific type

## NuGet Packages

Add packages using the `#r` directive at the top:

```csharp
#r "nuget: Newtonsoft.Json, 13.0.3"
#r "nuget: RestSharp, 110.2.0"

using Newtonsoft.Json;
using RestSharp;

public class Script
{
    public static object Main(string url)
    {
        var client = new RestClient(url);
        var request = new RestRequest();
        var response = client.Get(request);
        return JsonConvert.DeserializeObject(response.Content);
    }
}
```
