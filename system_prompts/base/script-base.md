# Windmill Script Writing Guide

## General Principles

- A script's inputs are its parameters. Credentials and configuration come in as resource-typed parameters, never hard-coded or read from the environment; the language section below shows how that language declares parameters
- When no resource type exists for a service, still take its connection as one resource-typed parameter, of a type holding the connection fields (base URL, account identifiers, credentials), its secret in a variable the resource references. A plain string parameter or a variable read inside the code only when the user asks for it: the resource keeps a connection's fields together, lets whoever runs the script pick an existing connection, and keeps the secret out of the arguments a run records. If looking up resource types reports that the instance is missing the Windmill Hub's types, mention that a superadmin can sync them, since the hub has types for most services. Otherwise, or when the user prefers it to syncing, propose the resource type and ask before creating it
- Libraries are installed automatically - do not show installation instructions
- In a language with an entrypoint function (TypeScript, Python, Go, Rust, PHP, R, …), name it `main` (`Main` in C#) and do not call it; in TypeScript it must be async. SQL, GraphQL, Bash, PowerShell and Ansible scripts have no `main`: their language section shows how they take arguments
- Where the language has a Windmill client (`wmill`), use it to interact with the platform
- A script's input schema may carry a top-level `prompt_for_ai` string: its author's instructions to an AI choosing the inputs. Follow it when you pick arguments to run that script, and keep it when you rewrite the schema

## Return Values

- A script can return any JSON-serializable value; a SQL script returns the rows its query produces
- Return values become available to subsequent flow steps via `results.step_id`

## Preprocessor Scripts

Preprocessor scripts process raw trigger data from various sources (webhook, custom HTTP route, SQS, WebSocket, Kafka, NATS, MQTT, AMQP, Postgres, GCP Pub/Sub, Azure, or email) before passing it to the flow. This separates the trigger logic from the flow logic and keeps the auto-generated UI clean.

A preprocessor is written in TypeScript or Python: its function is named `preprocessor` instead of `main`, and it receives a single parameter called `event` (the language section gives its type).

The returned object determines the parameter values passed to the flow.
e.g., `{ b: 1, a: 2 }` calls the flow with `a = 2` and `b = 1`, assuming the flow has two inputs called `a` and `b`.
