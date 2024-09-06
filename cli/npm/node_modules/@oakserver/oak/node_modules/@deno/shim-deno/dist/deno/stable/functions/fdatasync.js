"use strict";
///<reference path="../lib.deno.d.ts" />
Object.defineProperty(exports, "__esModule", { value: true });
exports.fdatasync = void 0;
const fs_1 = require("fs");
const util_1 = require("util");
const _fdatasync = (0, util_1.promisify)(fs_1.fdatasync);
exports.fdatasync = _fdatasync;
