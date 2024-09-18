# Generate windmill-client bundle

```bash
./node_modules/.bin/esbuild  src/index.ts --b
undle --outfile=windmill.js  --format=esm
```

# Generate d.ts bundle

node_modules/dts-bundle-generator/dist/bin/dts-bundle-generator.js -o
windmill.d.ts types/in dex.d.ts

# Develop client locally

`./dev.nu watch`

> If something not working, try to put //nobundle inside script body
