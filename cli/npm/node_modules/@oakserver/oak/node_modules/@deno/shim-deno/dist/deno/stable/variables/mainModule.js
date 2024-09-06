"use strict";
var _a, _b;
Object.defineProperty(exports, "__esModule", { value: true });
exports.mainModule = void 0;
///<reference path="../lib.deno.d.ts" />
const path_1 = require("path");
const url_1 = require("url");
exports.mainModule = (0, url_1.pathToFileURL)((_b = (_a = require.main) === null || _a === void 0 ? void 0 : _a.filename) !== null && _b !== void 0 ? _b : (0, path_1.join)(__dirname, "$deno$repl.ts")).href;
