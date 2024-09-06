"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.create = void 0;
const open_js_1 = require("./open.js");
const create = async function create(path) {
    return await (0, open_js_1.open)(path, { write: true, create: true, truncate: true });
};
exports.create = create;
