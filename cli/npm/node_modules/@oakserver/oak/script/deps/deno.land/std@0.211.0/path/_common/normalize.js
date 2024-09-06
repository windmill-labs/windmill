"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.assertArg = void 0;
const assert_path_js_1 = require("./assert_path.js");
function assertArg(path) {
    (0, assert_path_js_1.assertPath)(path);
    if (path.length === 0)
        return ".";
}
exports.assertArg = assertArg;
