"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toBase64 = void 0;
const constants_browser_1 = require("./constants.browser");
function toBase64(input) {
    let str = "";
    for (let i = 0; i < input.length; i += 3) {
        let bits = 0;
        let bitLength = 0;
        for (let j = i, limit = Math.min(i + 3, input.length); j < limit; j++) {
            bits |= input[j] << ((limit - j - 1) * constants_browser_1.bitsPerByte);
            bitLength += constants_browser_1.bitsPerByte;
        }
        const bitClusterCount = Math.ceil(bitLength / constants_browser_1.bitsPerLetter);
        bits <<= bitClusterCount * constants_browser_1.bitsPerLetter - bitLength;
        for (let k = 1; k <= bitClusterCount; k++) {
            const offset = (bitClusterCount - k) * constants_browser_1.bitsPerLetter;
            str += constants_browser_1.alphabetByValue[(bits & (constants_browser_1.maxLetterValue << offset)) >> offset];
        }
        str += "==".slice(0, 4 - bitClusterCount);
    }
    return str;
}
exports.toBase64 = toBase64;
