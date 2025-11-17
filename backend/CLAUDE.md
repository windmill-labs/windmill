# Backend Development (Rust)

## Core Principles

- Follow @rust-best-practices.mdc for detailed guidelines
- Database schema reference: @summarized_schema.txt
- The API routes prefixes are all listed in windmill-api/src/lib.rs

## Adding New Features

1. Update database schema with migration if necessary
2. Update backend/windmill-api/openapi.yaml after modifying API endpoints

## Querying the Database

To query the database directly, use psql with the following connection string:

```bash
psql postgres://postgres:changeme@localhost:5432/windmill
```

This can be helpful for:
- Inspecting database state during development
- Testing queries before implementing them in Rust
- Debugging data-related issues
