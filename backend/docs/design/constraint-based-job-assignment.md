# Constraint-Based Job Assignment

## Overview

This document explores adding constraint-based job assignment to Windmill, allowing workers to declare properties and jobs to specify constraints that must be satisfied.

## Current System

Jobs are assigned based on a single `tag` column in `v2_job_queue`. Workers declare which tags they handle, and pull jobs using:

```sql
SELECT id FROM v2_job_queue
WHERE running = false
  AND tag IN ('tag1', 'tag2', ...)
  AND scheduled_for <= now()
ORDER BY priority DESC NULLS LAST, scheduled_for
FOR UPDATE SKIP LOCKED
LIMIT 1
```

This is highly efficient due to:
- B-tree index on `(priority DESC, scheduled_for, tag) WHERE running = false`
- Simple `IN` clause allows index-only scans
- `FOR UPDATE SKIP LOCKED` for race-free claiming

## Proposed: Constraint Tags

### Concept

Instead of modifying the query structure, encode constraints as tags using a canonical hash/string format. This preserves the existing efficient query pattern.

### Constraints

- **Limit**: At most 3 global properties can be defined (e.g., `gpu`, `mem`, `region`)
- **Matching**: Always AND with strict equality
- Workers compute all 2³ = 8 possible constraint tag combinations they can satisfy

### Example

**Global properties defined**: `gpu`, `mem`, `region`

**Worker declares properties**:
```json
{
  "gpu": "a100",
  "mem": "high"
}
```

**Worker generates constraint tags** (all combinations it can satisfy):
```
c:                          # no constraints
c:gpu=a100
c:mem=high
c:gpu=a100,mem=high
```

Note: Worker doesn't generate tags with `region` because it doesn't have that property.

**Job with constraints**:
```json
{
  "gpu": "a100",
  "mem": "high"
}
```

**Job gets tag**: `c:gpu=a100,mem=high`

The existing `tag IN (...)` query matches this job to the worker.

### Tag Format

Constraint tags use a canonical format:
- Prefix: `c:` to distinguish from regular tags
- Key-value pairs: `key=value`
- Separator: `,` between pairs
- **Alphabetical ordering**: Keys are always sorted alphabetically for canonical representation

Examples:
- No constraints: `c:`
- Single constraint: `c:gpu=a100`
- Multiple constraints: `c:gpu=a100,mem=high` (gpu before mem alphabetically)

### Coexistence with Language Tags

Two options:

**Option A: Separate tag dimension**
- Jobs have both `tag` (language) and `constraint_tag`
- Query: `WHERE tag IN (...) AND (constraint_tag IS NULL OR constraint_tag IN (...))`
- Requires schema change and new index

**Option B: Combined tags**
- Combine language and constraints: `deno,c:gpu=a100,mem=high`
- Workers generate cartesian product of language tags × constraint tags
- No schema change, but more tags per worker

**Recommendation**: Option A is cleaner for separation of concerns.

## Implementation Plan

### 1. Schema Changes

```sql
-- Add constraint_tag column
ALTER TABLE v2_job_queue ADD COLUMN constraint_tag VARCHAR;

-- Add index for constraint-aware queries
CREATE INDEX queue_sort_constraint ON v2_job_queue
  (priority DESC NULLS LAST, scheduled_for, tag, constraint_tag)
  WHERE running = false;
```

### 2. Worker Configuration

Extend `WorkerConfig` to include constraint properties:

```rust
// windmill-common/src/worker.rs

pub struct WorkerConfig {
    pub worker_tags: Vec<String>,
    pub priority_tags_sorted: Vec<PriorityTags>,
    // New field
    pub constraint_properties: HashMap<String, String>,
}
```

### 3. Constraint Tag Generation

```rust
// windmill-common/src/constraints.rs

use itertools::Itertools;

/// Global constraint property keys (max 3)
pub const CONSTRAINT_KEYS: &[&str] = &["gpu", "mem", "region"];

/// Generate all constraint tags a worker can satisfy
pub fn generate_worker_constraint_tags(
    properties: &HashMap<String, String>
) -> Vec<String> {
    let mut tags = Vec::new();

    // Get the properties this worker has (filtered to valid keys)
    let worker_props: Vec<(&str, &str)> = CONSTRAINT_KEYS
        .iter()
        .filter_map(|&key| {
            properties.get(key).map(|v| (key, v.as_str()))
        })
        .collect();

    // Generate all 2^n combinations (power set)
    for size in 0..=worker_props.len() {
        for combo in worker_props.iter().combinations(size) {
            let tag = format_constraint_tag(&combo);
            tags.push(tag);
        }
    }

    tags
}

/// Generate the constraint tag for a job
pub fn generate_job_constraint_tag(
    constraints: &HashMap<String, String>
) -> String {
    let mut pairs: Vec<(&str, &str)> = constraints
        .iter()
        .filter(|(k, _)| CONSTRAINT_KEYS.contains(&k.as_str()))
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    // Sort alphabetically for canonical form
    pairs.sort_by_key(|(k, _)| *k);

    format_constraint_tag(&pairs.iter().map(|(k, v)| (k, v)).collect::<Vec<_>>())
}

fn format_constraint_tag(pairs: &[(&str, &str)]) -> String {
    if pairs.is_empty() {
        "c:".to_string()
    } else {
        let inner = pairs
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .join(",");
        format!("c:{}", inner)
    }
}
```

### 4. Query Generation Update

```rust
// windmill-common/src/worker.rs

pub fn make_pull_query(tags: &[String], constraint_tags: &[String]) -> String {
    let tags_sql = tags.iter().map(|x| format!("'{x}'")).join(", ");
    let constraint_sql = constraint_tags.iter().map(|x| format!("'{x}'")).join(", ");

    format_pull_query(format!(
        "SELECT id
        FROM v2_job_queue
        WHERE running = false
          AND tag IN ({})
          AND (constraint_tag IS NULL OR constraint_tag IN ({}))
          AND scheduled_for <= now()
        ORDER BY priority DESC NULLS LAST, scheduled_for
        FOR UPDATE SKIP LOCKED
        LIMIT 1",
        tags_sql,
        constraint_sql
    ))
}
```

### 5. Job Creation Update

When creating a job with constraints, compute the constraint tag:

```rust
// windmill-queue/src/jobs.rs

// In push() or create_job():
let constraint_tag = if let Some(constraints) = &job_constraints {
    Some(generate_job_constraint_tag(constraints))
} else {
    None
};
```

## Performance Analysis

### Query Efficiency

- **No degradation**: The `constraint_tag IN (...)` clause uses the same efficient index pattern
- **Partial index**: `WHERE running = false` still applies
- **Tag count**: Workers have at most `|language_tags| + 8` tags (8 constraint combinations max)

### Index Considerations

With the composite index on `(priority, scheduled_for, tag, constraint_tag)`:
- PostgreSQL can efficiently filter on all columns
- The `OR constraint_tag IS NULL` handles jobs without constraints

### Scalability

- 3 properties = 8 constraint tags per worker (manageable)
- 4 properties = 16 constraint tags (still reasonable)
- 5+ properties = consider alternative approaches

## Future Extensions

1. **OR support**: Could be added by generating multiple constraint tags for a single job
2. **Inequality constraints**: Would require different approach (not compatible with tag model)
3. **Dynamic properties**: Workers could update properties at runtime

## Open Questions

1. Should constraint properties be configured globally or per-workspace?
2. How to handle workers that don't declare any properties? (Treat as matching `c:` only)
3. Should there be validation that job constraints only use defined property keys?
