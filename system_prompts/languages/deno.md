# TypeScript (Deno)

Deno runtime with npm support via `npm:` prefix and native Deno libraries.

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function. Libraries are installed automatically.

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

Use the `RT` namespace for resource types:

```typescript
export async function main(stripe: RT.Stripe) {
  // stripe contains API key and config from the resource
}
```

Only use resource types if you need them to satisfy the instructions. Always use the RT namespace.

## Imports

```typescript
// npm packages use npm: prefix
import Stripe from "npm:stripe";
import { someFunction } from "npm:some-package";

// Deno standard library
import { serve } from "https://deno.land/std/http/server.ts";
```

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from "windmill-client";
```

See the SDK documentation for available methods.

## Preprocessor Scripts

For preprocessor scripts, the function should be named `preprocessor` and receives an `event` parameter:

```typescript
type Event = {
  kind:
    | "webhook"
    | "http"
    | "websocket"
    | "kafka"
    | "email"
    | "nats"
    | "postgres"
    | "sqs"
    | "mqtt"
    | "gcp";
  body: any;
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  return {
    param1: event.body.field1,
    param2: event.query.id,
  };
}
```

## S3 Object Operations

Windmill provides built-in support for S3-compatible storage operations.

### S3Object Type

The S3Object type represents a file in S3 storage:

```typescript
type S3Object = {
  s3: string; // Path within the bucket
};
```

## TypeScript Operations

```typescript
import * as wmill from "windmill-client";

// Load file content from S3
const content: Uint8Array = await wmill.loadS3File(s3object);

// Load file as stream
const blob: Blob = await wmill.loadS3FileStream(s3object);

// Write file to S3
const result: S3Object = await wmill.writeS3File(
  s3object, // Target path (or undefined to auto-generate)
  fileContent, // string or Blob
  s3ResourcePath // Optional: specific S3 resource to use
);
```
