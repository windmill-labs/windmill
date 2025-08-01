"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function parseYamlBoolean(input) {
    if (["true", "True", "TRUE"].lastIndexOf(input) >= 0) {
        return true;
    }
    else if (["false", "False", "FALSE"].lastIndexOf(input) >= 0) {
        return false;
    }
    throw `Invalid boolean "${input}"`;
}
exports.parseYamlBoolean = parseYamlBoolean;
function safeParseYamlInteger(input) {
    if (input.lastIndexOf('0o', 0) === 0) {
        return parseInt(input.substring(2), 8);
    }
    return parseInt(input);
}
function parseYamlInteger(input) {
    const result = safeParseYamlInteger(input);
    if (Number.isNaN(result)) {
        throw `Invalid integer "${input}"`;
    }
    return result;
}
exports.parseYamlInteger = parseYamlInteger;
function parseYamlBigInteger(input) {
    const result = parseYamlInteger(input);
    if (result > Number.MAX_SAFE_INTEGER && input.lastIndexOf('0o', 0) === -1) {
        return BigInt(input);
    }
    return result;
}
exports.parseYamlBigInteger = parseYamlBigInteger;
function parseYamlFloat(input) {
    if ([".nan", ".NaN", ".NAN"].lastIndexOf(input) >= 0) {
        return NaN;
    }
    const infinity = /^([-+])?(?:\.inf|\.Inf|\.INF)$/;
    const match = infinity.exec(input);
    if (match) {
        return (match[1] === '-') ? -Infinity : Infinity;
    }
    const result = parseFloat(input);
    if (!isNaN(result)) {
        return result;
    }
    throw `Invalid float "${input}"`;
}
exports.parseYamlFloat = parseYamlFloat;
var ScalarType;
(function (ScalarType) {
    ScalarType[ScalarType["null"] = 0] = "null";
    ScalarType[ScalarType["bool"] = 1] = "bool";
    ScalarType[ScalarType["int"] = 2] = "int";
    ScalarType[ScalarType["float"] = 3] = "float";
    ScalarType[ScalarType["string"] = 4] = "string";
})(ScalarType = exports.ScalarType || (exports.ScalarType = {}));
function determineScalarType(node) {
    if (node === undefined) {
        return ScalarType.null;
    }
    if (node.doubleQuoted || !node.plainScalar || node['singleQuoted']) {
        return ScalarType.string;
    }
    const value = node.value;
    if (["null", "Null", "NULL", "~", ''].indexOf(value) >= 0) {
        return ScalarType.null;
    }
    if (value === null || value === undefined) {
        return ScalarType.null;
    }
    if (["true", "True", "TRUE", "false", "False", "FALSE"].indexOf(value) >= 0) {
        return ScalarType.bool;
    }
    const base10 = /^[-+]?[0-9]+$/;
    const base8 = /^0o[0-7]+$/;
    const base16 = /^0x[0-9a-fA-F]+$/;
    if (base10.test(value) || base8.test(value) || base16.test(value)) {
        return ScalarType.int;
    }
    const float = /^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$/;
    const infinity = /^[-+]?(\.inf|\.Inf|\.INF)$/;
    if (float.test(value) || infinity.test(value) || [".nan", ".NaN", ".NAN"].indexOf(value) >= 0) {
        return ScalarType.float;
    }
    return ScalarType.string;
}
exports.determineScalarType = determineScalarType;
//# sourceMappingURL=scalarInference.js.map