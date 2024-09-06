"use strict";
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
Object.defineProperty(exports, "__esModule", { value: true });
exports.assertPath = void 0;
function assertPath(path) {
    if (typeof path !== "string") {
        throw new TypeError(`Path must be a string. Received ${JSON.stringify(path)}`);
    }
}
exports.assertPath = assertPath;
