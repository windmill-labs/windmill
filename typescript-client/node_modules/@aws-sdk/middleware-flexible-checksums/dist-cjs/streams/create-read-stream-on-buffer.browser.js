"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.createReadStreamOnBuffer = void 0;
function createReadStreamOnBuffer(buffer) {
    return new Blob([buffer]).stream();
}
exports.createReadStreamOnBuffer = createReadStreamOnBuffer;
