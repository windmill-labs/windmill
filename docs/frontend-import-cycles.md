# Frontend chunk cycles

If the emitted chunks import each other in a cycle, the app crashes on load with errors
like `TypeError: be is not a constructor` or `Cannot read properties of undefined
(reading 'PREPROCESSOR')`, and the user sees SvelteKit's default **"500 / Internal
Error"** page with nothing in the server log.

The build fails on this now (`assertAcyclicChunks` in `frontend/vite.config.js`), so you
should never hit it in production again. This document explains the error if you do hit
it at build time.

## Why a chunk cycle crashes

A chunk's imports are all evaluated before its own body. When chunk A and chunk B import
each other, entering at A evaluates B's body while **none** of A's body has run. Every
binding B reads from A is still in its pre-initialization state:

- `function f() {}` — hoisted, so calling it *works*
- `var X = class {}` / `const X = {...}` — **`undefined`**

So B's module-scope code calls a working function that reaches for an uninitialized class
and dies. Which member evaluates first depends on which route pulled the chunks in, which
is why the symptom is route-dependent, looks random, and often disappears on a second page
load. It is **not** a stale-asset or cache problem — new asset hashes do not change the
graph.

Anything a module does at evaluation time is exposed:

```ts
const toolDef = createToolDef(schema, 'x', '...') // calls z.toJSONSchema -> new JSONSchemaGenerator
const ids = { a: SPECIAL_MODULE_IDS.PREPROCESSOR } // reads an imported const
```

Making those lazy one at a time does not work — every module-scope read across the cycle
is a separate landmine, and a new one appears with the next refactor. Break the cycle.

## An acyclic module graph is not enough

This is the part that is easy to get wrong. Chunk *grouping* alone can create a cycle
where no import cycle exists between modules.

That is exactly what happened on vite 8.2.0 (#10468). `src/lib/gen` is a generated client
that imports nothing outside itself — a leaf. But the bundler split it: `index.ts` and
`core/*` landed in one chunk, while the 12k-line `schemas.gen.ts` was grouped into a
chunk with 53 unrelated copilot/app modules. Since `index.ts` does
`export * from './schemas.gen'`, the otherwise-pure gen chunk gained an edge into that app
chunk, which imports back into the copilot core chunk. One split produced **40 chunk
cycles**, and the copilot tool modules — which call `createToolDef` at module scope — began
evaluating against an uninitialized zod `JSONSchemaGenerator`.

The fix is to keep that leaf whole, in `frontend/vite.config.js`:

```js
build: { rollupOptions: { output: {
  advancedChunks: { groups: [{ name: 'gen', test: /[\\/]src[\\/]lib[\\/]gen[\\/]/ }] }
} } }
```

A barrel file (`export * from ...`) over a large generated module is the shape to watch:
it gives every importer of the barrel an edge to the re-exported module, wherever the
bundler decides to put it.

## When the build fails

`assertAcyclicChunks` prints the cycle with each chunk's source modules:

```
Cyclic chunk imports — this ships a runtime crash:
  _app/immutable/chunks/BdWm9WXx.js
      lib/gen/core/ApiError.ts, ..., lib/gen/index.ts
   ->
  _app/immutable/chunks/BUOJL1Np.js
      lib/gen/schemas.gen.ts, lib/components/copilot/chat/workspaceTools.ts, +42 more
   ->
  ...
```

Read it as: some module in chunk 1 imports some module in chunk 2, and so on back around.
To find the offending source edge, look for a module in one chunk that imports a module in
the next. Then, in order of preference:

1. **Make the import type-only** if only types are used — `import type` is erased and
   creates no runtime edge (so does a named import where every binding is `type`-prefixed,
   and `await import()` defers to a separate evaluation).
2. **Keep a self-contained leaf together** with an `advancedChunks` group, as above.
3. **Move the offending function to a new module** so the low-level module stops importing
   the high-level one. `lib/aiStore.ts` (AI model state) imported
   `components/copilot/lib.ts` (the AI client) for one call in `loadCopilot`, and the
   client imports the chat modules, which import `aiStore` back; extracting `loadCopilot`
   into `components/copilot/loadCopilot.ts` left `aiStore` a leaf.

## Reproducing a crash without the right route

Because the crash depends on which route enters the cycle first, it can be hard to trigger
by clicking. Force the bad entry order instead: build, serve `build/`, and dynamically
import the chunk that owns the shared bindings. If it is in a cycle, the import throws the
same error the user reports.

```html
<script type="module">
	try {
		await import('/_app/immutable/chunks/<shared-chunk>.js')
		document.title = 'OK'
	} catch (e) {
		document.title = 'THREW: ' + e.message
	}
</script>
```

Find the chunk by a preserved string literal — minification keeps them, e.g. zod's
`Error converting schema to JSON.` locates the chunk holding `JSONSchemaGenerator`.
