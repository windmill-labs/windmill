"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.test = void 0;
const definitions_js_1 = require("./definitions.js");
exports.test = Object.assign(function test() {
    handleDefinition(arguments);
}, {
    ignore() {
        handleDefinition(arguments, { ignore: true });
    },
    only() {
        handleDefinition(arguments, { only: true });
    },
});
function handleDefinition(args, additional) {
    var _a, _b;
    let testDef;
    const firstArg = args[0];
    const secondArg = args[1];
    const thirdArg = args[2];
    if (typeof firstArg === "string") {
        if (typeof secondArg === "object") {
            if (typeof thirdArg === "function") {
                if (secondArg.fn != null) {
                    throw new TypeError("Unexpected 'fn' field in options, test function is already provided as the third argument.");
                }
            }
            if (secondArg.name != null) {
                throw new TypeError("Unexpected 'name' field in options, test name is already provided as the first argument.");
            }
            // name, options, fn
            testDef = { name: firstArg, fn: thirdArg, ...secondArg };
        }
        else {
            // name, fn
            testDef = { name: firstArg, fn: secondArg };
        }
    }
    else if (firstArg instanceof Function) {
        // function only
        if (firstArg.name.length === 0) {
            throw new TypeError("The test function must have a name");
        }
        testDef = { fn: firstArg, name: firstArg.name };
        if (secondArg != null) {
            throw new TypeError("Unexpected second argument to Deno.test()");
        }
    }
    else if (typeof firstArg === "object") {
        testDef = { ...firstArg };
        if (typeof secondArg === "function") {
            // options, fn
            testDef.fn = secondArg;
            if (firstArg.fn != null) {
                throw new TypeError("Unexpected 'fn' field in options, test function is already provided as the second argument.");
            }
            if (testDef.name == null) {
                if (secondArg.name.length === 0) {
                    throw new TypeError("The test function must have a name");
                }
                // options without name, fn
                testDef.name = secondArg.name;
            }
        }
        else {
            if (typeof firstArg.fn !== "function") {
                throw new TypeError("Expected 'fn' field in the first argument to be a test function.");
            }
        }
    }
    else {
        throw new TypeError("Unknown test overload");
    }
    if (typeof testDef.fn !== "function") {
        throw new TypeError("Missing test function");
    }
    if (((_b = (_a = testDef.name) === null || _a === void 0 ? void 0 : _a.length) !== null && _b !== void 0 ? _b : 0) === 0) {
        throw new TypeError("The test name can't be empty");
    }
    if (additional === null || additional === void 0 ? void 0 : additional.ignore) {
        testDef.ignore = true;
    }
    if (additional === null || additional === void 0 ? void 0 : additional.only) {
        testDef.only = true;
    }
    definitions_js_1.testDefinitions.push(testDef);
}
