# Python Client Documentation

This document describes how the Python client documentation is generated and deployed, similar to the TypeScript client.

## Overview

The Python client uses **pdoc** to automatically generate API documentation from docstrings in the code, similar to how the TypeScript client uses TypeDoc.

## Architecture

Following the same pattern as the TypeScript client:

1. **Documentation Generator**: pdoc (Python) vs TypeDoc (TypeScript)
2. **Build Script**: `build_pdoc.sh` (similar to `build_typedoc.sh` for TS)
3. **Output Directory**: `docs/` containing static HTML
4. **Deployment**: Copied to `/frontend/static/pydocs/` during Docker build
5. **URL**: Accessible at `https://app.windmill.dev/pydocs/wmill.html`

## Building Documentation

### How It Works

The Python client documentation follows the same pattern as the TypeScript client:

1. **Docs are checked into git** - The `docs/` directory is committed to the repository
2. **Built during releases** - When `./build.sh` runs (on releases or manually), it calls `./build_pdoc.sh`
3. **Copied during Docker build** - The Dockerfile copies pre-built docs to the frontend

### Locally

To build/update the documentation:

```bash
cd /path/to/windmill/python-client
./build_pdoc.sh
```

This will:
1. Create a virtual environment if needed (`.venv/` - gitignored)
2. Install pdoc and dependencies
3. Generate HTML documentation in `./docs/`
4. Documentation will be available at `file://$(pwd)/docs/wmill.html`

After building, you should commit the updated docs:
```bash
git add docs/
git commit -m "Update Python client documentation"
```

### In CI/CD

The documentation is built automatically during the release process:

1. **On release** (`pypi_on_release.yml` workflow triggers on version tags)
2. Runs `./publish.sh` → calls `./build.sh` → calls `./build_pdoc.sh`
3. Docs are generated and should be committed separately or before the release

### In Docker Build

The documentation is copied during Docker build (see `Dockerfile` line 51):

```dockerfile
COPY /python-client/docs/ /frontend/static/pydocs/
```

This makes the docs available at `https://app.windmill.dev/pydocs/` in production.

## Documentation Structure

The generated documentation includes:

- **Main Module** (`wmill.html`): Overview and module-level functions
- **Client Class** (`wmill/client.html`): Full Windmill class API reference
- **S3 Types** (`wmill/s3_types.html`): S3 integration types and helpers
- **S3 Reader** (`wmill/s3_reader.html`): S3 file reading utilities

All documentation is automatically generated from:
- Function/class docstrings
- Type hints
- Parameter descriptions
- Return type annotations

## Writing Good Documentation

To maintain quality documentation:

1. **Use clear docstrings** following Google or NumPy style:
   ```python
   def my_function(param1: str, param2: int = 0) -> dict:
       """Short description of function.

       Longer description with more details about what the function does
       and any important notes.

       Args:
           param1: Description of param1
           param2: Description of param2 (default: 0)

       Returns:
           Description of return value

       Example:
           >>> result = my_function("test", 5)
           >>> print(result)
           {'status': 'ok'}
       """
   ```

2. **Add type hints** - pdoc uses them to generate better documentation
3. **Include examples** in docstrings where helpful
4. **Keep descriptions concise** but complete

## Comparison with TypeScript Client

| Aspect | TypeScript | Python |
|--------|-----------|--------|
| Generator | TypeDoc | pdoc |
| Build Script | `build_typedoc.sh` | `build_pdoc.sh` |
| Output Dir | `docs/` | `docs/` |
| Hosted At | `/tsdocs/` | `/pydocs/` |
| Entry Point | `modules.html` | `wmill.html` |

## References

- Windmill Docs: https://windmilldocs/docs/advanced/2_clients/python_client.md
- TypeScript Client Docs: https://app.windmill.dev/tsdocs/modules.html
- Python Client Docs: https://app.windmill.dev/pydocs/wmill.html
- pdoc Documentation: https://pdoc.dev/
