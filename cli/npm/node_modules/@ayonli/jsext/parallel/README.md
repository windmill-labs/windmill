# Parallel Threads

This file contains more details about the `parallel` module of this package.

## Use with Vite

### Not a problem in development mode

It may not be an obvious problem when using the `parallel()` function to link a
module in the development mode, and all things work normally. For example:

```ts
import parallel from "@ayonli/jsext/parallel";
const { default: didYouMean } = parallel(() => import("didyoumean"));

(async () => {
  const word = await didYouMean("helo", ["hello", "hola"]);
  console.log(word);
})();
```

This code works perfectly in development mode.

### Start to panic in production mode

However, after we run `npm run build` and generate bundled files, this code can
no longer run and will likely to throw some error saying that
`window is not defined` or `document is not defined`.

This is because, when bundling, even though Vite will bundle the `didyoumean`
module as a separate file, the file will import the entry file
`index-<hash>.js`, either for the use of some common functions, or for side
effects. Something like this:

```js
// didyoumean--h96y-lc7.js
import "./index-TFecDm-U.js";
```

The `index-TFecDm-U.js` will execute some IIFE functions that rely on the
`window` or `document` variable, which is not available in the worker thread,
thus causing the worker thread to panic.

### How to fix?

In order to fix this problem, we need to ensure that the generated file for the
worker thread will not import the entry file.

A workaround for this is to create a separate module for the use of worker
threads, and configure Vite to bundle its dependencies in a separate chunk that
exclude the use of `window` and `document` variable.

#### 1. Create an individual worker module

```ts
// utils/worker.ts
import _didYouMean from "didyoumean";

export const didYouMean = _didYouMean;
```

#### 2. Configure Vite to bundle worker dependencies separately

```ts
// vite.config.ts
import { defineConfig } from "vite";

export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (
            id.includes("tslib") || // This is usually required in TypeScript projects
            id.includes("didyoumean")
          ) {
            return "vendor";
          }
        },
      },
    },
  },
});
```

_NOTE: this separate chunk is used for the main entry `index-<hash>.js` as
well._

#### 3. Update the program to link the worker module

Now that we have a new module specifically for the use in the worker thread, we
need to update our program to use that module instead.

```ts
import parallel from "@ayonli/jsext/parallel";
const { didYouMean } = parallel(() => import("./utils/worker"));

(async () => {
  const word = await didYouMean("helo", ["hello", "hola"]);
  console.log(word);
})();
```

Now when running `npm run build`, Vite will generate a `vendor-<hash>.js` and a
`worker-<hash>.js`, where there is no `index-<hash>.js` included, our program
can now run as expected.

### Use with other bundlers

Other bundlers take about the same approach to fix the problem, the core
principle is to create separate chunk for the worker that doesn't rely on the
`browser` and `document` variable.
