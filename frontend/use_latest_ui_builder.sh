#!/bin/bash

# Auto-detect operating system
if [[ "$OSTYPE" == "darwin"* ]]; then
  IS_MAC=true
else
  IS_MAC=false
fi

cd ../../windmill-code-ui-builder
HASH=$(git rev-parse --short HEAD)
HASH=${HASH::-1}

echo "Using UI Builder hash: ${HASH}"

if [ "$IS_MAC" = true ]; then
  sed -i '' "s/ui_builder-[^.]*\.tar\.gz/ui_builder-${HASH}.tar.gz/" ../windmill/frontend/scripts/untar_ui_builder.js
else
  sed -i "s/ui_builder-[^.]*\.tar\.gz/ui_builder-${HASH}.tar.gz/" ../windmill/frontend/scripts/untar_ui_builder.js
fi
