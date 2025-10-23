# Windmill YAML Validator

A TypeScript-based YAML validator for Windmill flow files. This package validates flow.yaml files against the OpenFlow JSON schema to ensure they conform to the Windmill flow specification.

## Overview

The windmill-yaml-validator provides runtime validation for Windmill flow YAML files. It is currently used in the **Windmill VSCode extension** to show syntax errors and validation issues on `flow.yaml` files in real-time as developers edit them.

## Features

- **Schema-based validation**: Validates against the official OpenFlow JSON schema
- **Detailed error reporting**: Returns comprehensive error information with specific paths to invalid fields

## Installation

```bash
npm install windmill-yaml-validator
```

## Usage

### Basic Validation

```typescript
import { FlowValidator } from "windmill-yaml-validator";

const validator = new FlowValidator();

const yamlContent = `
summary: Test Flow
value:
  modules: []
`;

const result = validator.validateFlow(yamlContent);

if (result.errors.length === 0) {
  console.log("Flow is valid!");
} else {
  console.log("Validation errors:", result.errors);
}
```

### Error Handling

The validator returns detailed error information for invalid flows:

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

const result = validator.validateFlow(invalidYaml);

result.errors.forEach((error) => {
  console.log(`Error at ${error.instancePath}: ${error.message}`);
  // Example output:
  // Error at /summary: must be string
  // Error at /value/modules/0/value/language: must be equal to one of the allowed values
});
```

## API

### `FlowValidator`

Main validator class that validates Windmill flow YAML files.

#### Constructor

```typescript
new FlowValidator();
```

Creates a new validator instance. The constructor initializes the AJV validator with the OpenFlow schema.

#### Methods

##### `validateFlow(doc: string)`

Validates a flow document against the OpenFlow schema.

**Parameters:**

- `doc` (string): The YAML flow document as a string

**Returns:**

```typescript
{
  parsed: YamlParserResult<unknown>;  // Parsed YAML with source pointers
  errors: ErrorObject[];               // Array of validation errors (empty if valid)
}
```

**Throws:**

- Error if `doc` is not a string

## Development

### Building

```bash
npm run build
```

The build process:

1. Runs `gen_openflow_schema.sh` to generate the OpenFlow JSON schema from `openflow.openapi.yaml`
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

### Schema Generation

The validator uses a JSON schema generated from the OpenAPI specification:

```bash
./gen_openflow_schema.sh
```

This script:

- Bundles `openflow.openapi.yaml` into a single JSON schema
- Removes discriminator mappings for AJV compatibility
- Removes the `ToolValue` discriminator entirely (see below)
- Outputs to `src/gen/openflow.json`

#### Why Remove Discriminators?

The OpenFlow schema uses OpenAPI discriminators for efficient type resolution in `oneOf` schemas. However, AJV's discriminator support has limitations:

1. **Discriminator Mappings**: Not fully supported by AJV, so they are removed from all schemas
2. **ToolValue Discriminator**: Completely removed because `FlowModuleTool` uses `allOf` composition, which prevents AJV from finding the discriminator property (`tool_type`) at the expected location

**Impact**: Without discriminators, AJV falls back to standard `oneOf` validation, which:

- Tests each alternative until one matches
- Is slightly slower but still performant for our use case
- Provides the same validation correctness
- Works correctly with complex schema compositions like `allOf`
