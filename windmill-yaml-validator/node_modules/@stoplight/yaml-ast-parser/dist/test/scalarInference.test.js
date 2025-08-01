"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const chai = require("chai");
const assert = chai.assert;
const scalarInference_1 = require("../src/scalarInference");
const Yaml = require("../src/index");
suite('determineScalarType', () => {
    function determineScalarType(scalar) {
        return scalarInference_1.determineScalarType(scalar);
    }
    function safeLoad(input) {
        return Yaml.safeLoad(input, {});
    }
    let _test = test;
    suite('Plain Tag Resolution', () => {
        function test(name, type, acceptable) {
            _test(name, function () {
                for (const word of acceptable) {
                    assert.strictEqual(determineScalarType(safeLoad(word)), type, word);
                }
            });
        }
        ;
        test('boolean', scalarInference_1.ScalarType.bool, ["true", "True", "TRUE", "false", "False", "FALSE"]);
        test("null", scalarInference_1.ScalarType.null, ["null", "Null", "NULL", "~", ""]);
        _test("null as from an array", function () {
            const node = Yaml.newScalar('');
            node.plainScalar = true;
            assert.strictEqual(determineScalarType(node), scalarInference_1.ScalarType.null, "unquoted empty string");
        });
        test("integer", scalarInference_1.ScalarType.int, ["0", "0o7", "0x3A", "-19"]);
        test("float", scalarInference_1.ScalarType.float, ["0.", "-0.0", ".5", "+12e03", "-2E+05"]);
        test("float-infinity", scalarInference_1.ScalarType.float, [".inf", "-.Inf", "+.INF"]);
        test("float-NaN", scalarInference_1.ScalarType.float, [".nan", ".NaN", ".NAN"]);
        test("string", scalarInference_1.ScalarType.string, ["'true'", "TrUe", "nULl", "''", "'0'", '"1"', '" .5"', ".inF", ".nAn"]);
    });
    suite('Flow style', () => {
        test('still recognizes types', function () {
            const node = safeLoad(`[ null,
  true,
  0,
  0.,
  .inf,
  .nan,
  "-123\n345"
]`);
            const expected = [scalarInference_1.ScalarType.null, scalarInference_1.ScalarType.bool, scalarInference_1.ScalarType.int, scalarInference_1.ScalarType.float, scalarInference_1.ScalarType.float, scalarInference_1.ScalarType.float, scalarInference_1.ScalarType.string];
            assert.deepEqual(node.items.map(d => determineScalarType(d)), expected);
        });
    });
    suite('Block styles', () => {
        var variations = ['>', '|', '>8', '|+1', '>-', '>+', '|-', '|+'];
        test('are always strings', function () {
            for (const variant of variations) {
                assert.deepEqual(determineScalarType(safeLoad(variant + "\n 123")), scalarInference_1.ScalarType.string);
            }
        });
    });
});
suite('parseYamlInteger', () => {
    test('decimal', function () {
        assert.strictEqual(scalarInference_1.parseYamlInteger("0"), 0);
        assert.strictEqual(scalarInference_1.parseYamlInteger("-19"), -19);
        assert.strictEqual(scalarInference_1.parseYamlInteger("+1"), 1);
    });
    test('hexadecimal', function () {
        assert.strictEqual(scalarInference_1.parseYamlInteger("0x3A"), 58);
    });
    test('octal', function () {
        assert.strictEqual(scalarInference_1.parseYamlInteger("0o7"), 7);
    });
    test('otherwise', function () {
        let error;
        try {
            scalarInference_1.parseYamlInteger("'1'");
        }
        catch (e) {
            error = e;
        }
        assert(error, "should have thrown");
    });
});
suite('parseYamlBigInteger', () => {
    test('decimal', function () {
        assert.strictEqual(scalarInference_1.parseYamlBigInteger("0"), 0);
        assert.strictEqual(scalarInference_1.parseYamlBigInteger("-19"), -19);
        assert.strictEqual(scalarInference_1.parseYamlBigInteger("+1"), 1);
    });
    test('large decimal', function () {
        assert.equal(scalarInference_1.parseYamlBigInteger("549755813888"), BigInt("549755813888"));
        assert.equal(scalarInference_1.parseYamlBigInteger("9223372036854775807"), BigInt("9223372036854775807"));
    });
    test('hexadecimal', function () {
        assert.strictEqual(scalarInference_1.parseYamlBigInteger("0x3A"), 58);
    });
    test('octal', function () {
        assert.strictEqual(scalarInference_1.parseYamlBigInteger("0o7"), 7);
    });
    test('otherwise', function () {
        let error;
        try {
            scalarInference_1.parseYamlBigInteger("'1'");
        }
        catch (e) {
            error = e;
        }
        assert(error, "should have thrown");
    });
});
suite('parseYamlBoolean', () => {
    test('true', function () {
        for (const value of ["true", "True", "TRUE"]) {
            assert.strictEqual(scalarInference_1.parseYamlBoolean(value), true, value);
        }
    });
    test('false', function () {
        for (const value of ["false", "False", "FALSE"]) {
            assert.strictEqual(scalarInference_1.parseYamlBoolean(value), false, value);
        }
    });
    test('otherwise', function () {
        let error;
        try {
            scalarInference_1.parseYamlBoolean("tRUE");
        }
        catch (e) {
            error = e;
        }
        assert(error, "should have thrown");
    });
});
suite('parseYamlFloat', () => {
    test('float', function () {
        const values = ["0.", "-0.0", ".5", "+12e03", "-2E+05"];
        const expected = [0, -0, 0.5, 12000, -200000];
        for (var index = 0; index < values.length; index++) {
            assert.strictEqual(scalarInference_1.parseYamlFloat(values[index]), expected[index]);
        }
    });
    test('NaN', function () {
        for (const value of [".nan", ".NaN", ".NAN"]) {
            assert(isNaN(scalarInference_1.parseYamlFloat(value)), `isNaN(${value})`);
        }
    });
    test('infinity', function () {
        assert.strictEqual(scalarInference_1.parseYamlFloat(".inf"), Infinity);
        assert.strictEqual(scalarInference_1.parseYamlFloat("-.Inf"), -Infinity);
        assert.strictEqual(scalarInference_1.parseYamlFloat(".INF"), Infinity);
    });
    test('otherwise', function () {
        let error;
        try {
            scalarInference_1.parseYamlFloat("text");
        }
        catch (e) {
            error = e;
        }
        assert(error, "should have thrown");
    });
});
//# sourceMappingURL=scalarInference.test.js.map