cd ../../../windmill-code-ui-builder
HASH=$(git rev-parse --short HEAD)
HASH=${HASH::-1}

sed -i "s/ui_builder-[^.]*\.tar\.gz/ui_builder-${HASH}.tar.gz/" ../git/windmill/frontend/scripts/untar_ui_builder.js
