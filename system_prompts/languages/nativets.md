# TypeScript (Native)

Native TypeScript execution with fetch only - no external imports allowed.

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

**No imports allowed.** Use the globally available `fetch` function:

```typescript
export async function main(url: string) {
  const response = await fetch(url);
  return await response.json();
}
```

## Windmill Client

The windmill client is not available in native TypeScript mode. Use fetch to call APIs directly.

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
    param2: event.query.id
  };
}
```
