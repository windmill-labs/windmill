# Windmill YAML Validator

A TypeScript-based YAML validator for Windmill flow, schedule, and trigger files.

## Overview

The windmill-yaml-validator provides runtime validation for Windmill YAML files. It is used by editor integrations to show validation errors while editing:

- `flow.yaml` / `flow.yml`
- `*.schedule.yaml` / `*.schedule.yml`
- `*.{http|websocket|kafka|nats|postgres|mqtt|sqs|gcp}_trigger.yaml` (or `.yml`)

## Features

- **Unified validation API**: One validator class for flow/schedule/trigger files
- **Schema-based validation**: Uses OpenFlow and backend OpenAPI-derived schemas
- **Detailed error reporting**: Returns comprehensive error information with specific paths to invalid fields

## Installation

```bash
npm install windmill-yaml-validator
```

## Usage

### Basic Validation

```typescript
import { WindmillYamlValidator } from "windmill-yaml-validator";

const validator = new WindmillYamlValidator();

const flowYaml = `
summary: Test Flow
value:
  modules: []
`;

const flowResult = validator.validate(flowYaml, { type: "flow" });

const scheduleYaml = `
schedule: "0 0 12 * * *"
timezone: "UTC"
enabled: true
script_path: "f/jobs/daily_sync"
is_flow: false
`;

const scheduleResult = validator.validate(scheduleYaml, { type: "schedule" });

const triggerYaml = `
script_path: "f/triggers/http_handler"
is_flow: false
route_path: "api/webhook"
request_type: "sync"
authentication_method: "none"
http_method: "post"
is_static_website: false
workspaced_route: false
wrap_body: false
raw_string: false
`;

const triggerResult = validator.validate(triggerYaml, {
  type: "trigger",
  triggerKind: "http",
});

console.log(flowResult.errors, scheduleResult.errors, triggerResult.errors);
```

### Target Inference by Filename

```typescript
import {
  WindmillYamlValidator,
  getValidationTargetFromFilename,
} from "windmill-yaml-validator";

const validator = new WindmillYamlValidator();
const target = getValidationTargetFromFilename(
  "f/webhooks/order_created.http_trigger.yaml"
);

if (target) {
  const result = validator.validate(fileContents, target);
  console.log(result.errors);
}
```

### Error Handling

```typescript
const invalidYaml = `
summary: 123  # Should be a string
value:
  modules:
    - id: step1
      value:
        type: rawscript
        language: invalid_language  # Invalid enum value
`;

const result = validator.validate(invalidYaml, { type: "flow" });

result.errors.forEach((error) => {
  console.log(`Error at ${error.instancePath}: ${error.message}`);
  // Example output:
  // Error at /summary: must be string
  // Error at /value/modules/0/value/language: must be equal to one of the allowed values
});
```

## API

### `WindmillYamlValidator`

Main validator class for Windmill YAML validation.

#### Constructor

```typescript
new WindmillYamlValidator();
```

Initializes AJV validators for flow, schedule, and trigger schemas.

#### Methods

##### `validate(doc: string, target: ValidationTarget)`

Validates a YAML document against the selected target schema.

**Parameters:**

- `doc` (string): YAML document string
- `target` (`ValidationTarget`):
  - `{ type: "flow" }`
  - `{ type: "schedule" }`
  - `{ type: "trigger", triggerKind: "http" | "websocket" | "kafka" | "nats" | "postgres" | "mqtt" | "sqs" | "gcp" | "email" }`

**Returns:**

```typescript
{
  parsed: YamlParserResult<unknown>;  // Parsed YAML with source pointers
  errors: ErrorObject[];               // Array of validation errors (empty if valid)
}
```

**Throws:**

- Error if `doc` is not a string

### `getValidationTargetFromFilename(path: string)`

Infers validation target from file naming conventions. Returns `null` for unsupported files.

## Development

### Building

```bash
npm run build
```

The build process:

1. Runs `gen_openflow_schema.sh` to generate:
   - `src/gen/openflow.json`
   - `src/gen/schedule.json`
   - `src/gen/triggers/*.json`
2. Removes discriminator mappings (not supported by AJV)
3. Compiles TypeScript to JavaScript

### Testing

```bash
npm test
```

Run tests in watch mode:

```bash
npm test:watch
```

### Testing locally with the CLI

To test local changes before publishing, use `npm link`:

```bash
# In windmill-yaml-validator/
npm run build
npm link

# In cli/
npm link windmill-yaml-validator
bun run src/main.ts lint
```

### Schema Generation

The validator uses a JSON schema generated from the OpenAPI specification:

```bash
./gen_openflow_schema.sh
```

This script:

- Converts `openflow.openapi.yaml` and `backend/windmill-api/openapi.yaml` into JSON
- Removes discriminator mappings for AJV compatibility
- Removes the `ToolValue` discriminator entirely (see below)
- Generates standalone schedule/trigger schemas for CLI file shape

#### Why Remove Discriminators?

The OpenFlow schema uses OpenAPI discriminators for efficient type resolution in `oneOf` schemas. However, AJV's discriminator support has limitations:

1. **Discriminator Mappings**: Not fully supported by AJV, so they are removed from all schemas
2. **ToolValue Discriminator**: Completely removed because `FlowModuleTool` uses `allOf` composition, which prevents AJV from finding the discriminator property (`tool_type`) at the expected location

**Impact**: Without discriminators, AJV falls back to standard `oneOf` validation, which:

- Tests each alternative until one matches
- Is slightly slower but still performant for our use case
- Provides the same validation correctness
- Works correctly with complex schema compositions like `allOf`

## Breaking Change

`FlowValidator` and `validateFlow()` were replaced by `WindmillYamlValidator` and `validate(doc, target)`.
