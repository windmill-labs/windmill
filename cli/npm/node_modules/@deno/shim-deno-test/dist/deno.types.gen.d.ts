/// <reference types="node" />
import { URL } from "url";
/**
 * Register a test which will be run when `deno test` is used on the command
 * line and the containing module looks like a test module.
 *
 * `fn` can be async if required.
 *
 * ```ts
 * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
 *
 * Deno.test({
 *   name: "example test",
 *   fn() {
 *     assertEquals("world", "world");
 *   },
 * });
 *
 * Deno.test({
 *   name: "example ignored test",
 *   ignore: Deno.build.os === "windows",
 *   fn() {
 *     // This test is ignored only on Windows machines
 *   },
 * });
 *
 * Deno.test({
 *   name: "example async test",
 *   async fn() {
 *     const decoder = new TextDecoder("utf-8");
 *     const data = await Deno.readFile("hello_world.txt");
 *     assertEquals(decoder.decode(data), "Hello world");
 *   }
 * });
 * ```
 *
 * @category Testing
 */
export declare const test: DenoTest;
/** @category Testing */
export interface DenoTest {
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required.
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test({
     *   name: "example test",
     *   fn() {
     *     assertEquals("world", "world");
     *   },
     * });
     *
     * Deno.test({
     *   name: "example ignored test",
     *   ignore: Deno.build.os === "windows",
     *   fn() {
     *     // This test is ignored only on Windows machines
     *   },
     * });
     *
     * Deno.test({
     *   name: "example async test",
     *   async fn() {
     *     const decoder = new TextDecoder("utf-8");
     *     const data = await Deno.readFile("hello_world.txt");
     *     assertEquals(decoder.decode(data), "Hello world");
     *   }
     * });
     * ```
     *
     * @category Testing
     */
    (t: TestDefinition): void;
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required.
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test("My test description", () => {
     *   assertEquals("hello", "hello");
     * });
     *
     * Deno.test("My async test description", async () => {
     *   const decoder = new TextDecoder("utf-8");
     *   const data = await Deno.readFile("hello_world.txt");
     *   assertEquals(decoder.decode(data), "Hello world");
     * });
     * ```
     *
     * @category Testing
     */
    (name: string, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required. Declared function must have a name.
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test(function myTestName() {
     *   assertEquals("hello", "hello");
     * });
     *
     * Deno.test(async function myOtherTestName() {
     *   const decoder = new TextDecoder("utf-8");
     *   const data = await Deno.readFile("hello_world.txt");
     *   assertEquals(decoder.decode(data), "Hello world");
     * });
     * ```
     *
     * @category Testing
     */
    (fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required.
     *
     * ```ts
     * import {assert, fail, assertEquals} from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test("My test description", { permissions: { read: true } }, (): void => {
     *   assertEquals("hello", "hello");
     * });
     *
     * Deno.test("My async test description", { permissions: { read: false } }, async (): Promise<void> => {
     *   const decoder = new TextDecoder("utf-8");
     *   const data = await Deno.readFile("hello_world.txt");
     *   assertEquals(decoder.decode(data), "Hello world");
     * });
     * ```
     *
     * @category Testing
     */
    (name: string, options: Omit<TestDefinition, "fn" | "name">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required.
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test(
     *   {
     *     name: "My test description",
     *     permissions: { read: true },
     *   },
     *   () => {
     *     assertEquals("hello", "hello");
     *   },
     * );
     *
     * Deno.test(
     *   {
     *     name: "My async test description",
     *     permissions: { read: false },
     *   },
     *   async () => {
     *     const decoder = new TextDecoder("utf-8");
     *     const data = await Deno.readFile("hello_world.txt");
     *     assertEquals(decoder.decode(data), "Hello world");
     *   },
     * );
     * ```
     *
     * @category Testing
     */
    (options: Omit<TestDefinition, "fn" | "name">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Register a test which will be run when `deno test` is used on the command
     * line and the containing module looks like a test module.
     *
     * `fn` can be async if required. Declared function must have a name.
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test(
     *   { permissions: { read: true } },
     *   function myTestName() {
     *     assertEquals("hello", "hello");
     *   },
     * );
     *
     * Deno.test(
     *   { permissions: { read: false } },
     *   async function myOtherTestName() {
     *     const decoder = new TextDecoder("utf-8");
     *     const data = await Deno.readFile("hello_world.txt");
     *     assertEquals(decoder.decode(data), "Hello world");
     *   },
     * );
     * ```
     *
     * @category Testing
     */
    (options: Omit<TestDefinition, "fn">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(t: Omit<TestDefinition, "ignore">): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(name: string, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(name: string, options: Omit<TestDefinition, "fn" | "name" | "ignore">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(options: Omit<TestDefinition, "fn" | "name" | "ignore">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for ignoring a particular test case.
     *
     * @category Testing
     */
    ignore(options: Omit<TestDefinition, "fn" | "ignore">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(t: Omit<TestDefinition, "only">): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(name: string, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(name: string, options: Omit<TestDefinition, "fn" | "name" | "only">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(options: Omit<TestDefinition, "fn" | "name" | "only">, fn: (t: TestContext) => void | Promise<void>): void;
    /**
     * Shorthand property for focusing a particular test case.
     *
     * @category Testing
     */
    only(options: Omit<TestDefinition, "fn" | "only">, fn: (t: TestContext) => void | Promise<void>): void;
}
/** @category Testing */
export interface TestDefinition {
    fn: (t: TestContext) => void | Promise<void>;
    /** The name of the test. */
    name: string;
    /**
     * If truthy the current test step will be ignored.
     *
     * It is a quick way to skip over a step, but also can be used for
     * conditional logic, like determining if an environment feature is present.
     */
    ignore?: boolean;
    /**
     * If at least one test has `only` set to `true`, only run tests that have
     * `only` set to `true` and fail the test suite.
     */
    only?: boolean;
    /**
     * Check that the number of async completed operations after the test step
     * is the same as number of dispatched operations. This ensures that the
     * code tested does not start async operations which it then does
     * not await. This helps in preventing logic errors and memory leaks
     * in the application code.
     *
     * @default {true}
     */
    sanitizeOps?: boolean;
    /**
     * Ensure the test step does not "leak" resources - like open files or
     * network connections - by ensuring the open resources at the start of the
     * test match the open resources at the end of the test.
     *
     * @default {true}
     */
    sanitizeResources?: boolean;
    /**
     * Ensure the test case does not prematurely cause the process to exit,
     * for example via a call to {@linkcode Deno.exit}.
     *
     * @default {true}
     */
    sanitizeExit?: boolean;
    /**
     * Specifies the permissions that should be used to run the test.
     *
     * Set this to "inherit" to keep the calling runtime permissions, set this
     * to "none" to revoke all permissions, or set a more specific set of
     * permissions using a {@linkcode PermissionOptionsObject}.
     *
     * @default {"inherit"}
     */
    permissions?: PermissionOptions;
}
/**
 * Context that is passed to a testing function, which can be used to either
 * gain information about the current test, or register additional test
 * steps within the current test.
 *
 * @category Testing
 */
export interface TestContext {
    /** The current test name. */
    name: string;
    /** The string URL of the current test. */
    origin: string;
    /**
     * If the current test is a step of another test, the parent test context
     * will be set here.
     */
    parent?: TestContext;
    /**
     * Run a sub step of the parent test or step. Returns a promise
     * that resolves to a boolean signifying if the step completed successfully.
     *
     * The returned promise never rejects unless the arguments are invalid.
     *
     * If the test was ignored the promise returns `false`.
     *
     * ```ts
     * Deno.test({
     *   name: "a parent test",
     *   async fn(t) {
     *     console.log("before the step");
     *     await t.step({
     *       name: "step 1",
     *       fn(t) {
     *         console.log("current step:", t.name);
     *       }
     *     });
     *     console.log("after the step");
     *   }
     * });
     * ```
     */
    step(definition: TestStepDefinition): Promise<boolean>;
    /**
     * Run a sub step of the parent test or step. Returns a promise
     * that resolves to a boolean signifying if the step completed successfully.
     *
     * The returned promise never rejects unless the arguments are invalid.
     *
     * If the test was ignored the promise returns `false`.
     *
     * ```ts
     * Deno.test(
     *   "a parent test",
     *   async (t) => {
     *     console.log("before the step");
     *     await t.step(
     *       "step 1",
     *       (t) => {
     *         console.log("current step:", t.name);
     *       }
     *     );
     *     console.log("after the step");
     *   }
     * );
     * ```
     */
    step(name: string, fn: (t: TestContext) => void | Promise<void>): Promise<boolean>;
    /**
     * Run a sub step of the parent test or step. Returns a promise
     * that resolves to a boolean signifying if the step completed successfully.
     *
     * The returned promise never rejects unless the arguments are invalid.
     *
     * If the test was ignored the promise returns `false`.
     *
     * ```ts
     * Deno.test(async function aParentTest(t) {
     *   console.log("before the step");
     *   await t.step(function step1(t) {
     *     console.log("current step:", t.name);
     *   });
     *   console.log("after the step");
     * });
     * ```
     */
    step(fn: (t: TestContext) => void | Promise<void>): Promise<boolean>;
}
/** @category Testing */
export interface TestStepDefinition {
    /**
     * The test function that will be tested when this step is executed. The
     * function can take an argument which will provide information about the
     * current step's context.
     */
    fn: (t: TestContext) => void | Promise<void>;
    /** The name of the step. */
    name: string;
    /**
     * If truthy the current test step will be ignored.
     *
     * This is a quick way to skip over a step, but also can be used for
     * conditional logic, like determining if an environment feature is present.
     */
    ignore?: boolean;
    /**
     * Check that the number of async completed operations after the test step
     * is the same as number of dispatched operations. This ensures that the
     * code tested does not start async operations which it then does
     * not await. This helps in preventing logic errors and memory leaks
     * in the application code.
     *
     * Defaults to the parent test or step's value.
     */
    sanitizeOps?: boolean;
    /**
     * Ensure the test step does not "leak" resources - like open files or
     * network connections - by ensuring the open resources at the start of the
     * step match the open resources at the end of the step.
     *
     * Defaults to the parent test or step's value.
     */
    sanitizeResources?: boolean;
    /**
     * Ensure the test step does not prematurely cause the process to exit,
     * for example via a call to {@linkcode Deno.exit}.
     *
     * Defaults to the parent test or step's value.
     */
    sanitizeExit?: boolean;
}
/**
 * A set of options which can define the permissions within a test or worker
 * context at a highly specific level.
 *
 * @category Permissions
 */
export interface PermissionOptionsObject {
    /**
     * Specifies if the `env` permission should be requested or revoked.
     * If set to `"inherit"`, the current `env` permission will be inherited.
     * If set to `true`, the global `env` permission will be requested.
     * If set to `false`, the global `env` permission will be revoked.
     *
     * @default {false}
     */
    env?: "inherit" | boolean | string[];
    /**
     * Specifies if the `sys` permission should be requested or revoked.
     * If set to `"inherit"`, the current `sys` permission will be inherited.
     * If set to `true`, the global `sys` permission will be requested.
     * If set to `false`, the global `sys` permission will be revoked.
     *
     * @default {false}
     */
    sys?: "inherit" | boolean | string[];
    /**
     * Specifies if the `hrtime` permission should be requested or revoked.
     * If set to `"inherit"`, the current `hrtime` permission will be inherited.
     * If set to `true`, the global `hrtime` permission will be requested.
     * If set to `false`, the global `hrtime` permission will be revoked.
     *
     * @default {false}
     */
    hrtime?: "inherit" | boolean;
    /**
     * Specifies if the `net` permission should be requested or revoked.
     * if set to `"inherit"`, the current `net` permission will be inherited.
     * if set to `true`, the global `net` permission will be requested.
     * if set to `false`, the global `net` permission will be revoked.
     * if set to `string[]`, the `net` permission will be requested with the
     * specified host strings with the format `"<host>[:<port>]`.
     *
     * @default {false}
     *
     * Examples:
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test({
     *   name: "inherit",
     *   permissions: {
     *     net: "inherit",
     *   },
     *   async fn() {
     *     const status = await Deno.permissions.query({ name: "net" })
     *     assertEquals(status.state, "granted");
     *   },
     * });
     * ```
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test({
     *   name: "true",
     *   permissions: {
     *     net: true,
     *   },
     *   async fn() {
     *     const status = await Deno.permissions.query({ name: "net" });
     *     assertEquals(status.state, "granted");
     *   },
     * });
     * ```
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test({
     *   name: "false",
     *   permissions: {
     *     net: false,
     *   },
     *   async fn() {
     *     const status = await Deno.permissions.query({ name: "net" });
     *     assertEquals(status.state, "denied");
     *   },
     * });
     * ```
     *
     * ```ts
     * import { assertEquals } from "https://deno.land/std/testing/asserts.ts";
     *
     * Deno.test({
     *   name: "localhost:8080",
     *   permissions: {
     *     net: ["localhost:8080"],
     *   },
     *   async fn() {
     *     const status = await Deno.permissions.query({ name: "net", host: "localhost:8080" });
     *     assertEquals(status.state, "granted");
     *   },
     * });
     * ```
     */
    net?: "inherit" | boolean | string[];
    /**
     * Specifies if the `ffi` permission should be requested or revoked.
     * If set to `"inherit"`, the current `ffi` permission will be inherited.
     * If set to `true`, the global `ffi` permission will be requested.
     * If set to `false`, the global `ffi` permission will be revoked.
     *
     * @default {false}
     */
    ffi?: "inherit" | boolean | Array<string | URL>;
    /**
     * Specifies if the `read` permission should be requested or revoked.
     * If set to `"inherit"`, the current `read` permission will be inherited.
     * If set to `true`, the global `read` permission will be requested.
     * If set to `false`, the global `read` permission will be revoked.
     * If set to `Array<string | URL>`, the `read` permission will be requested with the
     * specified file paths.
     *
     * @default {false}
     */
    read?: "inherit" | boolean | Array<string | URL>;
    /**
     * Specifies if the `run` permission should be requested or revoked.
     * If set to `"inherit"`, the current `run` permission will be inherited.
     * If set to `true`, the global `run` permission will be requested.
     * If set to `false`, the global `run` permission will be revoked.
     *
     * @default {false}
     */
    run?: "inherit" | boolean | Array<string | URL>;
    /**
     * Specifies if the `write` permission should be requested or revoked.
     * If set to `"inherit"`, the current `write` permission will be inherited.
     * If set to `true`, the global `write` permission will be requested.
     * If set to `false`, the global `write` permission will be revoked.
     * If set to `Array<string | URL>`, the `write` permission will be requested with the
     * specified file paths.
     *
     * @default {false}
     */
    write?: "inherit" | boolean | Array<string | URL>;
}
/**
 * Options which define the permissions within a test or worker context.
 *
 * `"inherit"` ensures that all permissions of the parent process will be
 * applied to the test context. `"none"` ensures the test context has no
 * permissions. A `PermissionOptionsObject` provides a more specific
 * set of permissions to the test context.
 *
 * @category Permissions
 */
export type PermissionOptions = "inherit" | "none" | PermissionOptionsObject;
