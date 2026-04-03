---
name: write-script-rlang
description: MUST use when writing R scripts.
---

## CLI Commands

Place scripts in a folder. After writing, tell the user they can run:
- `wmill generate-metadata` - Generate .script.yaml and .lock files
- `wmill sync push` - Deploy to Windmill

Do NOT run these commands yourself. Instead, inform the user that they should run them.

Use `wmill resource-type list --schema` to discover available resource types.

# R

## Structure

Define a `main` function using `<-` or `=` assignment. Parameters become the script inputs:

```r
library(dplyr)
library(jsonlite)

main <- function(x, name = "default", flag = TRUE) {
    df <- tibble(x = x, name = name)
    result <- df %>% mutate(greeting = paste("Hello", name))
    return(toJSON(result, auto_unbox = TRUE))
}
```

**Important:**
- The `main` function is required
- Use `library()` to load packages — they are resolved and installed automatically
- `jsonlite` is always available (used internally for argument parsing)
- Return values must be JSON-serializable

## Parameters

R types map to Windmill types:
- `numeric` → float/int
- `character` → string
- `logical` → bool (use `TRUE`/`FALSE`)
- `list` → object/dict
- `NULL` → null

Default values are inferred from the function signature:

```r
main <- function(
    name,              # required string
    count = 10,        # optional int, default 10
    verbose = FALSE    # optional bool, default FALSE
) {
    # ...
}
```

## Resources and Variables

Use the built-in Windmill helpers (no import needed):

```r
main <- function() {
    # Get a variable
    api_key <- get_variable("f/my_folder/api_key")

    # Get a resource (returns a list)
    db <- get_resource("f/my_folder/postgres_config")
    host <- db$host
    port <- db$port

    return(list(host = host, port = port))
}
```

## Output

Return any JSON-serializable value from `main`. The return value becomes the step result:

```r
main <- function(x) {
    # Return a scalar
    return(x + 1)

    # Or a list (becomes JSON object)
    return(list(result = x + 1, status = "ok"))
}
```

## Annotations

Control execution behavior with comment annotations:

```r
#renv_verbose = true        # Show verbose renv output during resolution
#renv_install_verbose = true # Show verbose output during package installation
#sandbox = true              # Run in nsjail sandbox (requires nsjail)
```
