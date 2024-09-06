import { Pipeline } from '../pipe.js';

function pipe(fn, ...args) {
    if ([String, Number, BigInt, Boolean, Symbol].includes(this.constructor)) {
        return new Pipeline(this.valueOf()).pipe(fn, ...args);
    }
    else {
        return new Pipeline(this).pipe(fn, ...args);
    }
}
String.prototype.pipe = pipe;
Number.prototype.pipe = pipe;
BigInt.prototype.pipe = pipe;
Boolean.prototype.pipe = pipe;
Symbol.prototype.pipe = pipe;
Date.prototype.pipe = pipe;
RegExp.prototype.pipe = pipe;
Error.prototype.pipe = pipe;
Map.prototype.pipe = pipe;
Set.prototype.pipe = pipe;
Array.prototype.pipe = pipe;
Int8Array.prototype.pipe = pipe;
Uint8Array.prototype.pipe = pipe;
Uint8ClampedArray.prototype.pipe = pipe;
Int16Array.prototype.pipe = pipe;
Uint16Array.prototype.pipe = pipe;
Int32Array.prototype.pipe = pipe;
Uint32Array.prototype.pipe = pipe;
Float32Array.prototype.pipe = pipe;
Float64Array.prototype.pipe = pipe;
BigInt64Array.prototype.pipe = pipe;
BigUint64Array.prototype.pipe = pipe;
ArrayBuffer.prototype.pipe = pipe;
if (typeof SharedArrayBuffer === "function") {
    SharedArrayBuffer.prototype.pipe = pipe;
}
if (typeof Blob === "function") {
    Blob.prototype.pipe = pipe;
}
if (typeof Event === "function") {
    Event.prototype.pipe = pipe;
}
//# sourceMappingURL=pipe.js.map
