"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.mkdir = void 0;
const promises_1 = require("fs/promises");
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const variables_js_1 = require("../variables.js");
const mkdir = async function mkdir(path, options) {
    try {
        await (0, promises_1.mkdir)(path, options);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "EEXIST") {
            throw new variables_js_1.errors.AlreadyExists(`File exists (os error 17), mkdir '${path}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.mkdir = mkdir;
