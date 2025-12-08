# Eval Results - 2025-12-08T16-07-57.975Z

## User Prompt
```
THIS IS A TEST, CODE SHOULD BE MINIMAL FUNCTIONING CODE, IF WE NEED RETURN VALUES RETURN EXAMPLE VALUES

STEP 1: Fetch mock users from api
STEP 2: Filter only active users:
STEP 3: Loop on all users
STEP 4: Do branches based on user's role, do different action based on that
STEP 5: Return action taken for each user
```

## Results

| Variant | Success | Total Tokens | Tool Calls | Iterations |
|---------|---------|--------------|------------|------------|
| baseline | true | 541235 | 12 | 13 |
| no-full-schema | true | 242456 | 16 | 12 |

## Flow Outputs

- baseline: ./baseline.json
  - Flow definition: ./baseline_flow.json
- no-full-schema: ./no-full-schema.json
  - Flow definition: ./no-full-schema_flow.json