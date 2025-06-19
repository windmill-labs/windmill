# Backend Development (Rust)

## Core Principles

- Follow @rust-best-practices.mdc for detailed guidelines
- Database schema reference: @summarized_schema.txt

## Windmill-Specific Patterns

- Workspace isolation: All operations are workspace-scoped
- Job execution: Jobs go through the queue system
- Permission checks: Use existing auth middleware
- Audit logging: Log all significant operations

## Adding New Features

1. Update database schema with migration
2. Add API endpoint in appropriate module
3. Update openapi.yaml
