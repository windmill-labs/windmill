"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.chdir = void 0;
const url_1 = require("url");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const variables_js_1 = require("../variables.js");
const chdir = function (path) {
    try {
        return process.chdir(path instanceof URL ? (0, url_1.fileURLToPath)(path) : path);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new variables_js_1.errors.NotFound(`No such file or directory (os error 2), chdir '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.chdir = chdir;
