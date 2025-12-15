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
