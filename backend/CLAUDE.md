# Backend Development (Rust)

## Core Principles

- Follow @rust-best-practices.mdc for detailed guidelines
- Database schema reference: @summarized_schema.txt
- The API routes prefixes are all listed in windmill-api/src/lib.rs
- This repository is the open source side of the project. The enterprise files (\*\_ee.rs) are in the `windmill-ee-private` folder (a sibling directory). Those files are symlinked into their corresponding locations within each crate's `src/` directory.

## JSON Handling

- **Prefer `Box<serde_json::value::RawValue>` over `serde_json::Value`** when possible, especially:
  - When storing JSON in the database (JSONB columns)
  - When passing JSON through without modification
  - When the JSON structure doesn't need to be inspected or manipulated
- This avoids unnecessary parsing/serialization overhead and preserves the original JSON format
- Use `serde_json::Value` only when you need to inspect, modify, or construct JSON programmatically

## Adding New Features

1. Update database schema with migration if necessary
2. Update backend/windmill-api/openapi.yaml after modifying API endpoints
