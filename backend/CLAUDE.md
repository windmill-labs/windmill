# Backend Development (Rust)

## Project Structure

Windmill uses a workspace-based architecture with multiple crates:

- **windmill-api**: API server functionality
- **windmill-worker**: Job execution
- **windmill-common**: Shared code used by all crates
- **windmill-queue**: Job & flow queuing
- **windmill-audit**: Audit logging
- Other specialized crates (git-sync, autoscaling, etc.)

## Key References

- Database schema: @summarized_schema.txt
- API route prefixes: `windmill-api/src/lib.rs`

## Adding New Code

### Module Organization

- Place new code in the appropriate crate based on functionality
- For API endpoints, create or modify files in `windmill-api/src/` organized by domain
- For shared functionality, use `windmill-common/src/`
- Follow existing patterns for file structure and organization

### API Endpoints

- Follow existing patterns in the `windmill-api` crate
- Use axum's routing system and extractors
- Update `backend/windmill-api/openapi.yaml` after modifying API endpoints

### Database Changes

- Update database schema with migration if necessary
- Use `sqlx` for database operations with prepared statements
- Use transactions for multi-step operations

## Enterprise Features

- Enterprise files use the `*_ee.rs` suffix
- Enterprise source is in `windmill-ee-private` folder (sibling directory), symlinked into each crate's `src/`
- Use feature flags: `#[cfg(feature = "enterprise")]`
- Isolate enterprise code in separate modules

## Testing

- Write unit tests for core functionality
- Use the `#[cfg(test)]` module for test code
- For database tests, use the existing test utilities

## Common Crates

- **tokio**: Async runtime
- **axum**: Web server and routing
- **sqlx**: Database operations
- **serde**: Serialization/deserialization
- **tracing**: Logging and diagnostics
- **reqwest**: HTTP client
