---
name: write-script-go
description: MUST use when writing Go scripts.
---

## CLI Commands

Place scripts in a folder. After writing, run:
- `wmill script generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Use `wmill resource-type list --schema` to discover available resource types.

# Go

## Structure

The file package must be `inner` and export a function called `main`:

```go
package inner

func main(param1 string, param2 int) (map[string]interface{}, error) {
    return map[string]interface{}{
        "result": param1,
        "count":  param2,
    }, nil
}
```

**Important:**
- Package must be `inner`
- Return type must be `({return_type}, error)`
- Function name is `main` (lowercase)

## Return Types

The return type can be any Go type that can be serialized to JSON:

```go
package inner

type Result struct {
    Name  string `json:"name"`
    Count int    `json:"count"`
}

func main(name string, count int) (Result, error) {
    return Result{
        Name:  name,
        Count: count,
    }, nil
}
```

## Error Handling

Return errors as the second return value:

```go
package inner

import "errors"

func main(value int) (string, error) {
    if value < 0 {
        return "", errors.New("value must be positive")
    }
    return "success", nil
}
```
