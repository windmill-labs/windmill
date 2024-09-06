"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.watchFs = void 0;
const promises_1 = require("fs/promises");
const path_1 = require("path");
const iterutil_js_1 = require("../../internal/iterutil.js");
const watchFs = function watchFs(paths, options = { recursive: true }) {
    paths = Array.isArray(paths) ? paths : [paths];
    const ac = new AbortController();
    const { signal } = ac;
    // TODO(mkr): create valid rids for watchers
    const rid = -1;
    const masterWatcher = (0, iterutil_js_1.merge)(paths.map((path) => (0, iterutil_js_1.mapAsync)((0, iterutil_js_1.filterAsync)((0, promises_1.watch)(path, { recursive: options === null || options === void 0 ? void 0 : options.recursive, signal }), (info) => info.filename != null), (info) => ({
        kind: "modify",
        paths: [(0, path_1.resolve)(path, info.filename)],
    }))));
    function close() {
        ac.abort();
    }
    return Object.assign(masterWatcher, {
        rid,
        close,
        [Symbol.dispose]: close,
    });
};
exports.watchFs = watchFs;
