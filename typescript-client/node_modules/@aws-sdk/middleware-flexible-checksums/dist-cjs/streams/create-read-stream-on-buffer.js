"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createReadStreamOnBuffer = void 0;
const stream_1 = require("stream");
function createReadStreamOnBuffer(buffer) {
    const stream = new stream_1.Transform();
    stream.push(buffer);
    stream.push(null);
    return stream;
}
exports.createReadStreamOnBuffer = createReadStreamOnBuffer;
