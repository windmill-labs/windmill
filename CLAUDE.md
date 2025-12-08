# Windmill Development Guide

## Overview

Windmill is an open-source developer platform for building internal tools, workflows, API integrations, background jobs, workflows, and user interfaces. See @windmill-overview.mdc for full platform details.

## New Feature Implementation Guidelines

When implementing new features in Windmill, follow these best practices:

- **Clean Code First**: Write clean, readable, and maintainable code. Prioritize clarity over cleverness.
- **Avoid Duplication at All Costs**: Before writing new code, thoroughly search for existing implementations that can be reused or extended.
- **Adapt Existing Code**: Refactor and generalize existing code when necessary to avoid logic duplication. Extract common patterns into reusable utilities.
- **Follow Established Patterns**: Study existing code patterns in the codebase and maintain consistency with established conventions.
- **Single Responsibility**: Each function, component, and module should have a single, well-defined responsibility.
- **Incremental Implementation**: Break large features into smaller, reviewable chunks that can be implemented and tested incrementally.

## Language-Specific Guides

- Backend (Rust): @backend/rust-best-practices.mdc + @backend/summarized_schema.txt
- Frontend (Svelte 5): @frontend/svelte5-best-practices.mdc

## Querying the Database

To query the database directly, use psql with the following connection string:

```bash
psql postgres://postgres:changeme@localhost:5432/windmill
```

This can be helpful for:

- Inspecting database state during development
- Testing queries before implementing them in Rust
- Debugging data-related issues
