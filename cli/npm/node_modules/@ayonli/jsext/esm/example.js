import { isDeno, isNodeLike } from './env.js';

/**
 * Writes unit tests as if writing examples, inspired by Golang.
 * @module
 * @deprecated It turns out that this module isn't really helpful, and has
 * compatibility issues with Bun, **tsx** and browsers, it will be removed in
 * the future.
 */
/**
 * Inspired by Golang's **Example as Test** design, creates a function that
 * carries example code with `// output:` comments, when the returned function
 * is called, it will automatically check if the actual output matches the one
 * declared in the comment.
 *
 * The example function receives a customized `console` object which will be
 * used to log outputs instead of using the built-in `console`.
 *
 * **NOTE:**
 * This function is used to simplify the process of writing tests, currently,
 * it does not work in Bun, **tsx** and browsers, because Bun hasn't implement
 * the `Console` constructor and removes comments during runtime, **tsx** also
 * remove comments, and the function depends on Node.js built-in modules.
 *
 * @example
 * ```ts
 * import example from "@ayonli/jsext/example";
 *
 * it("should output as expected", example(console => {
 *     console.log("Hello, World!");
 *     // output:
 *     // Hello, World!
 * }));
 * ```
 *
 * @deprecated It turns out that this function isn't really helpful, and has
 * compatibility issues with Bun, **tsx** and browsers, it will be removed in
 * the future.
 */
function example(fn, options = undefined) {
    const call = {};
    Error.captureStackTrace(call, example);
    return async function (...args) {
        const fnStr = fn.toString();
        let lines = fnStr.split("\n").slice(1, -1);
        let offset = lines.findIndex(line => {
            return line.trim().toLowerCase() === "// output:";
        });
        if (offset === -1) {
            // no output is detected, skip the function
            return;
        }
        else {
            offset += 1;
            lines = lines.slice(offset);
        }
        if (lines.findIndex(line => {
            return line.trim().toLowerCase() === "// output:";
        }) !== -1) {
            throw new Error("there can only be one output comment in the example");
        }
        let expected = [];
        for (let line of lines) {
            line = line.trimStart();
            if (line.startsWith("//")) {
                if (line[2] && line[2] !== " ") {
                    throw new Error("the output comment must start with '// '");
                }
                expected.push(line.slice(3));
            }
            else {
                throw new Error("the output comment must be at the end of the example");
            }
        }
        // remove empty tailing lines
        const _expected = [...expected];
        expected = [];
        for (let i = _expected.length - 1; i >= 0; i--) {
            if (_expected[i] !== "") {
                expected.push(_expected[i]);
            }
        }
        expected.reverse();
        const assert = await import('node:assert');
        const { Writable } = await import('node:stream');
        const { Console } = await import('node:console');
        const logs = [];
        const decoder = new TextDecoder();
        const stdout = new Writable({
            write(chunk, _, callback) {
                logs.push(chunk);
                // const str = decoder.decode(chunk);
                // const lines = str.split("\n");
                // lines.forEach((line, i) => {
                //     if (line || i !== lines.length - 1) {
                //         logs.push(line);
                //     }
                // });
                callback();
            },
        });
        const _console = new Console(stdout);
        const returns = fn.call(this, _console, ...args);
        const handleResult = async () => {
            var _a;
            const actual = logs.map(chunk => decoder.decode(chunk))
                .join("\n")
                .replace(/[\n]+$/, "");
            const _expected = expected.join("\n");
            try {
                // @ts-ignore
                assert.ok(actual === _expected, `\nexpected:\n${_expected}\n\ngot:\n${actual}`);
                if (!(options === null || options === void 0 ? void 0 : options.suppress)) {
                    for (const chunk of logs) {
                        if (isDeno) {
                            await Deno.stdout.write(chunk);
                        }
                        else if (isNodeLike) {
                            await new Promise(resolve => {
                                process.stdout.write(chunk, () => resolve());
                            });
                        }
                    }
                }
            }
            catch (err) {
                Object.defineProperty(err, "stack", {
                    configurable: true,
                    writable: true,
                    enumerable: false,
                    value: err.stack
                        + "\n" + ((_a = call.stack) === null || _a === void 0 ? void 0 : _a.split("\n").slice(1).join("\n")),
                });
                throw err;
            }
        };
        if (typeof (returns === null || returns === void 0 ? void 0 : returns.then) === "function") {
            await Promise.resolve(returns);
        }
        await new Promise(resolve => stdout.end(() => resolve()));
        await handleResult();
    };
}

export { example as default };
//# sourceMappingURL=example.js.map
