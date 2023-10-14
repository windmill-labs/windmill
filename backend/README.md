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

### Compile sqlx for offline ci

```
cargo sqlx prepare --workspace -- --bin windmill --features enterprise
```
