---
name: write-script-graphql
description: MUST use when writing GraphQL queries.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# GraphQL

## Structure

Write GraphQL queries or mutations. Arguments can be added as query parameters:

```graphql
query GetUser($id: ID!) {
  user(id: $id) {
    id
    name
    email
  }
}
```

## Variables

Variables are passed as script arguments and automatically bound to the query:

```graphql
query SearchProducts($query: String!, $limit: Int = 10) {
  products(search: $query, first: $limit) {
    edges {
      node {
        id
        name
        price
      }
    }
  }
}
```

## Mutations

```graphql
mutation CreateUser($input: CreateUserInput!) {
  createUser(input: $input) {
    id
    name
    createdAt
  }
}
```
