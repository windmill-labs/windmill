#!/bin/bash
# Build Python documentation with pdoc
# Similar to typescript-client/build_typedoc.sh

set -e

# Create/activate virtual environment if needed
if [ ! -d ".venv" ]; then
    python3 -m venv .venv
    .venv/bin/pip install -q pdoc httpx
fi

# Install the package
.venv/bin/pip install -q ./wmill

# Generate documentation
.venv/bin/pdoc wmill -o docs

echo "Python documentation built successfully in ./docs/"
