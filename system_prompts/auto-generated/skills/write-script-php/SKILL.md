---
name: write-script-php
description: MUST use when writing PHP scripts.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — generate `.script.yaml` and `.lock` files for the script you modified.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill sync push` and `wmill generate-metadata` modify workspace state or local files — only run these when the user explicitly asks; otherwise tell them which to run.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

# PHP

## Structure

The script must start with `<?php` and contain at least one function called `main`:

```php
<?php

function main(string $param1, int $param2) {
    return ["result" => $param1, "count" => $param2];
}
```

## Resource Types

On Windmill, credentials and configuration are stored in resources and passed as parameters to main.

You need to **redefine** the type of the resources that are needed before the main function. Always check if the class already exists using `class_exists`:

```php
<?php

if (!class_exists('Postgresql')) {
    class Postgresql {
        public string $host;
        public int $port;
        public string $user;
        public string $password;
        public string $dbname;
    }
}

function main(Postgresql $db) {
    // $db contains the database connection details
}
```

The resource type name has to be exactly as specified.

## Library Dependencies

Specify library dependencies as comments before the main function:

```php
<?php

// require:
// guzzlehttp/guzzle
// stripe/stripe-php@^10.0

function main() {
    // Libraries are available
}
```

One dependency per line. No need to require autoload, it is already done.
