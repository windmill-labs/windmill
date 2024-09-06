import { Pipeline } from "../pipe.ts";
import { ValueOf } from "../types.ts";

export interface Pipeable {
    /**
     * Calls a function using the current value as its argument and returns a
     * new {@link Pipeline} instance that holds the result.
     */
    pipe<R, A extends any[] = any[]>(fn: (value: ValueOf<this>, ...args: A) => R, ...args: A): Pipeline<R>;
}

declare global {
    interface String extends Pipeable { }
    interface Number extends Pipeable { }
    interface BigInt extends Pipeable { }
    interface Boolean extends Pipeable { }
    interface Symbol extends Pipeable { }

    interface Date extends Pipeable { }
    interface RegExp extends Pipeable { }
    interface Error extends Pipeable { }

    interface Map<K, V> extends Pipeable { }
    interface Set<T> extends Pipeable { }

    interface Array<T> extends Pipeable { }
    interface ReadonlyArray<T> extends Pipeable { }
    interface TypedArray extends Pipeable { }
    interface Int8Array extends Pipeable { }
    interface Uint8Array extends Pipeable { }
    interface Uint8ClampedArray extends Pipeable { }
    interface Int16Array extends Pipeable { }
    interface Uint16Array extends Pipeable { }
    interface Int32Array extends Pipeable { }
    interface Uint32Array extends Pipeable { }
    interface Float32Array extends Pipeable { }
    interface Float64Array extends Pipeable { }
    interface BigInt64Array extends Pipeable { }
    interface BigUint64Array extends Pipeable { }

    interface ArrayBuffer extends Pipeable { }
    interface SharedArrayBuffer extends Pipeable { }
    interface Blob extends Pipeable { }

    interface Event extends Pipeable { }
}

function pipe<R, A extends any[] = any[]>(
    this: any,
    fn: (value: any, ...args: A) => R,
    ...args: A
): Pipeline<R> {
    if ([String, Number, BigInt, Boolean, Symbol].includes(this.constructor)) {
        return new Pipeline(this.valueOf()).pipe(fn, ...args);
    } else {
        return new Pipeline(this).pipe(fn, ...args);
    }
};

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
