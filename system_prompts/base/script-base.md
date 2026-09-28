# Windmill Script Writing Guide

## General Principles

- A script's inputs are its parameters. Credentials and configuration come in as resource-typed parameters, never hard-coded or read from the environment; the language section below shows how that language declares parameters
- Libraries are installed automatically - do not show installation instructions
- In a language with an entrypoint function (TypeScript, Python, Go, Rust, PHP, R, …), name it `main` and do not call it; in TypeScript it must be async. SQL, GraphQL, Bash and PowerShell scripts have no `main`: their language section shows how they take arguments
- Where the language has a Windmill client (`wmill`), use it to interact with the platform

## Return Values

- A script can return any JSON-serializable value; a SQL script returns the rows its query produces
- Return values become available to subsequent flow steps via `results.step_id`

## Preprocessor Scripts

Preprocessor scripts process raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, AMQP, Postgres, GCP Pub/Sub, Azure, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.

A preprocessor is written in TypeScript or Python: its function is named `preprocessor` instead of `main`, and it receives a single parameter called `event` (the language section gives its type).

The returned object determines the parameter values passed to the flow.
e.g., `{ b: 1, a: 2 }` calls the flow with `a = 2` and `b = 1`, assuming the flow has two inputs called `a` and `b`.
