# Frontend Development (Svelte 5)

## Core Principles

- Follow @svelte5-best-practices.mdc for detailed guidelines
- Use Runes ($state, $derived, $effect) for reactivity
- Keep components small and focused
- Always use keys in {#each} blocks

## UI Guidelines

- Follow existing design system
- Use consistent spacing and colors

## Backend API

- If you need to call the backend API, you can find the available routes in ../backend/windmill-api/openapi.yaml
- You can also use the associated types and services that are auto generated from the openapi file. They are in src/lib/gen/\*gen.ts files
