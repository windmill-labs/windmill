"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toBase64 = void 0;
const util_buffer_from_1 = require("@smithy/util-buffer-from");
const toBase64 = (input) => (0, util_buffer_from_1.fromArrayBuffer)(input.buffer, input.byteOffset, input.byteLength).toString("base64");
exports.toBase64 = toBase64;
