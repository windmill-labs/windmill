# Windmill Backend

This folder holds all backend components, the [src/](./src/) folder only
contains files used to build the "root" binary.

## Components

| name                                  | description                                                                                               |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| [windmill-api](./windmill-api/)       | The API server, exposing functionality to other components and the frontend                               |
| [windmill-audit](./windmill-audit/)   | Contains audit functionality, allowing different components to record important actions                   |
| [windmill-common](./windmill-common/) | Common code shared by all crates                                                                          |
| [windmill-queue](./windmill-queue/)   | Contains job & flow queuing functionality, commonly written to by the API server and read from by workers |
| [windmill-worker](./windmill-worker/) | The worker. Used to process and execute flows & jobs.                                                     |
| [parsers](./parsers/)                 | Contains code to parse signatures in different langauges.                                                 |

## Features

### Embedded PostgreSQL (`pg_embed`)

The `pg_embed` feature allows Windmill to run with an embedded PostgreSQL database, eliminating the need for a separate PostgreSQL instance. This is useful for local development, testing, or single-machine deployments.

**Building with pg_embed:**
```bash
cargo build --features pg_embed
```

**System Dependencies:**

The embedded PostgreSQL requires certain system libraries to be installed:

- **Arch Linux:**
  ```bash
  sudo pacman -S libxml2 icu openssl
  ```

- **Ubuntu/Debian:**
  ```bash
  sudo apt-get install libxml2 libicu-dev libssl-dev
  ```

- **RHEL/Fedora:**
  ```bash
  sudo dnf install libxml2 libicu openssl-libs
  ```

**Usage:**

When the `pg_embed` feature is enabled, if `DATABASE_URL` is not set, Windmill will automatically start an embedded PostgreSQL instance.

You can customize the embedded PostgreSQL behavior with these environment variables:
- `PG_EMBED_DATA_DIR`: Directory for PostgreSQL data (default: `./postgresql_data`)
- `PG_EMBED_PORT`: Port for PostgreSQL (default: `5432`)
- `PG_EMBED_DATABASE`: Database name (default: `windmill`)
- `PG_EMBED`: Set to any value to force embedded PostgreSQL even if DATABASE_URL exists

**Example:**
```bash
# Run with default embedded PostgreSQL
./windmill

# Or with custom settings
PG_EMBED_DATA_DIR=/my/data PG_EMBED_PORT=5433 ./windmill
```

### Compile sqlx for offline ci

```
cargo sqlx prepare --workspace -- --bin windmill --features enterprise
```
