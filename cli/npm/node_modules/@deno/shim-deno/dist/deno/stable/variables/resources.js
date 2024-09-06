"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.resources = void 0;
const resources = function resources() {
    console.warn([
        "Deno.resources() shim returns a dummy object that does not update.",
        "If you think this is a mistake, raise an issue at https://github.com/denoland/node_deno_shims/issues",
    ].join("\n"));
    return {};
};
exports.resources = resources;
