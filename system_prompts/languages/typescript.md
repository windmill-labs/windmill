# TypeScript (Bun/Deno/Native)

## Variants

- `bun`: Bun runtime, full npm ecosystem, fastest execution
- `deno`: Deno runtime, npm via `npm:{package}`, deno libraries supported
- `nativets`, `bunnative`: Native TypeScript, fetch only, no external imports allowed

## Structure

Export a single **async** function called `main`:

```typescript
export async function main(param1: string, param2: number) {
  // Your code here
  return { result: param1, count: param2 };
}
```

Do not call the main function.

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

### Bun Runtime
```typescript
import Stripe from 'stripe'
import { someFunction } from 'some-package'
```

### Deno Runtime
```typescript
import Stripe from 'npm:stripe'
import { someFunction } from 'npm:some-package'
// Or deno libraries:
import { serve } from 'https://deno.land/std/http/server.ts'
```

### Native TypeScript (nativets, bunnative)
No imports allowed. Use the globally available `fetch` function:
```typescript
export async function main(url: string) {
  const response = await fetch(url);
  return await response.json();
}
```

## Windmill Client

Import the windmill client for platform interactions:

```typescript
import * as wmill from 'windmill-client'
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
  body: any; // Raw body of the request
  headers: Record<string, string>;
  query: Record<string, string>;
};

export async function preprocessor(event: Event) {
  // Transform the event into flow input parameters
  return {
    param1: event.body.field1,
    param2: event.query.id
  };
}
```
