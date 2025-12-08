# Eval Results - 2025-12-08T15-32-16.820Z

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
| baseline | true | 541231 | 12 | 13 |
| no-full-schema | true | 340251 | 16 | 17 |

## Flow Outputs

- baseline: ./baseline.json
- no-full-schema: ./no-full-schema.json