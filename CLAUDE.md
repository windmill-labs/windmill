# Windmill Development Guide

## Overview

Windmill is an open-source developer platform for building internal tools, workflows, and APIs. See @windmill-overview.mdc for full platform details.

## Quick Start

- Backend: Rust with PostgreSQL
- Frontend: SvelteKit with TypeScript

## Development Rules

- **Follow existing patterns** - study similar code before implementing
- **Performance matters** - follow optimization guidelines in language-specific guides
- **Security first** - never commit secrets, always validate inputs

## Language-Specific Guides

- Backend (Rust): @backend/rust-best-practices.mdc + @backend/summarized_schema.txt
- Frontend (Svelte 5): @frontend/svelte5-best-practices.mdc

## Common Patterns

- Error handling: Use Result<T, Error> consistently
- Database: Always use transactions for multi-step operations
- API endpoints: Update openapi.yaml after changes
- UI components: Use runes for reactivity, keep components small
