"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
Object.defineProperty(exports, "__esModule", { value: true });
exports.assertArgs = void 0;
const assert_path_js_1 = require("./assert_path.js");
function assertArgs(from, to) {
    (0, assert_path_js_1.assertPath)(from);
    (0, assert_path_js_1.assertPath)(to);
    if (from === to)
        return "";
}
exports.assertArgs = assertArgs;
