# Windmill Utils Internal

Internal TypeScript utility package for Windmill development tools and scripts.

## What this package contains

This package provides internal utilities and tools used by the Windmill CLI, the VS CODE extension, and the frontend.

## Development

To work on this package we need to generate the windmill client, and remove .ts extensions to the imports and exports (added by default for deno compatibility, so that the CLI can use this package).

You just need to run this before working on the package:

```bash
npm run dev
```

After you are done with your modifications, add back the .ts extensions:

```bash
./remove-ts-ext.sh -r
```

### Building

To build the package for production:

```bash
npm run build
```

### Publishing

To publish the package:

```bash
./publish.sh
```