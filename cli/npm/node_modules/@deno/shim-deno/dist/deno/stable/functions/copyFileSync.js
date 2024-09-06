"use strict";
///<reference path="../lib.deno.d.ts" />
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.copyFileSync = void 0;
const fs = __importStar(require("fs"));
const errorMap_js_1 = __importDefault(require("../../internal/errorMap.js"));
const errors = __importStar(require("../variables/errors.js"));
const copyFileSync = (src, dest) => {
    try {
        fs.copyFileSync(src, dest);
    }
    catch (error) {
        if ((error === null || error === void 0 ? void 0 : error.code) === "ENOENT") {
            throw new errors.NotFound(`File not found, copy '${src}' -> '${dest}'`);
        }
        throw (0, errorMap_js_1.default)(error);
    }
};
exports.copyFileSync = copyFileSync;
