#!/bin/bash

VERSION=$1
echo "Updating versions to: $VERSION"

sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" backend/Cargo.toml
sed -i -e "/^export const VERSION =/s/= .*/= \"v$VERSION\";/" cli/main.ts
sed -i -e "/version: /s/: .*/: $VERSION/" backend/windmill-api/openapi.yaml
sed -i -e "/version: /s/: .*/: $VERSION/" openflow.openapi.yaml
sed -i -e "/\"version\": /s/: .*,/: \"$VERSION\",/" frontend/package.json
sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" python-client/wmill/pyproject.toml
sed -i -e "/^windmill-api =/s/= .*/= \"\\^$VERSION\"/" python-client/wmill/pyproject.toml
sed -i -e "/^version =/s/= .*/= \"$VERSION\"/" python-client/wmill_pg/pyproject.toml
# sed -i -e "/^wmill =/s/= .*/= \"\\^$VERSION\"/" python-client/wmill_pg/pyproject.toml
sed -i -e "/^wmill =/s/= .*/= \">=$VERSION\"/" lsp/Pipfile
sed -i -e "/^wmill_pg =/s/= .*/= \">=$VERSION\"/" lsp/Pipfile

sed -i -zE "s/name = \"windmill\"\nversion = \"[^\"]*\"\\n(.*)/name = \"windmill\"\nversion = \"$VERSION\"\\n\\1/" backend/Cargo.lock

cd frontend && npm i --package-lock-only
