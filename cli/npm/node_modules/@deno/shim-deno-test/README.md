# @deno/shim-deno-test

Subset of [@deno/shim-deno](https://www.npmjs.com/package/@deno/shim-deno) for
only `Deno.test`.

```ts
import { Deno, test, testDefinitions } from "@deno/shim-deno-test";

Deno.test("some test", () => {
  // test here
});

// or
test("some other test", () => {
  // test here
});

// read from testDefinitions here
testDefinitions.length === 2;
```

## Note - Not a Test Runner

This shim is not a test runner. It simply collects tests into the
`testDefinitions` array. The idea is that you run a module that does `Deno.test`
calls and then you splice out the test definitions from `testDefinitions` (ex.
`const definitions = testDefinitions.splice(0, testDefinitions.length)`) and
provide those to a test runner.
