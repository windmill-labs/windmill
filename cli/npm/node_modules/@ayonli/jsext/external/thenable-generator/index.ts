if (!Symbol.asyncIterator) {
    Object.defineProperty(Symbol, "asyncIterator", {
        value: Symbol("Symbol.asyncIterator")
    });
}

export const source: unique symbol = Symbol("GeneratorSource");
export const status: unique symbol = Symbol("GeneratorStatus");
export const result: unique symbol = Symbol("GeneratorResult");

export interface Catchable<T> {
    catch?<R = never>(
        onrejected?: (reason: any) => R | PromiseLike<R>
    ): Promise<T | R>;
}

export interface ThenableGeneratorLike<T = unknown, TReturn = any, TNext = unknown> extends PromiseLike<T>, Catchable<T>, Partial<Generator<T, TReturn, TNext>> { }

export interface ThenableAsyncGeneratorLike<T = unknown, TReturn = any, TNext = unknown> extends PromiseLike<T>, Catchable<T>, Partial<AsyncGenerator<T, TReturn, TNext>> { }

export type ThenableGeneratorFunction<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]> = (...args: TArgs) => ThenableGenerator<T, TReturn, TNext>;

export type ThenableAsyncGeneratorFunction<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]> = (...args: TArgs) => ThenableAsyncGenerator<T, TReturn, TNext>;

export interface ThenableGeneratorFunctionConstructor<T = unknown, TReturn = any, TNext = unknown> {
    (fn: Function): ThenableGeneratorFunction<T, TReturn, TNext> | ThenableAsyncGeneratorFunction<T, TReturn, TNext>;
    new(fn: Function): ThenableGeneratorFunction<T, TReturn, TNext> | ThenableAsyncGeneratorFunction<T, TReturn, TNext>;

    /**
     * @deprecated
     */
    create<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]>(
        fn: (...args: TArgs) => AsyncGenerator<T, TReturn, TNext> | AsyncIterable<T> | Promise<T>
    ): ThenableAsyncGeneratorFunction<T, TReturn, TNext, TArgs>;
    /**
     * @deprecated
     */
    create<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]>(
        fn: (...args: TArgs) => Generator<T, TReturn, TNext> | Iterable<T> | T
    ): ThenableGeneratorFunction<T, TReturn, TNext, TArgs>;
}

export class Thenable<T = any> implements PromiseLike<T>, Catchable<T> {
    protected [source]: any;
    protected [status]: "suspended" | "closed" | "erred";
    protected [result]: any;

    constructor(_source: any) {
        this[source] = _source;
        this[status] = "suspended";
        this[result] = void 0;
    }

    then<R1 = T, R2 = never>(
        onfulfilled?: ((value: T) => R1 | PromiseLike<R1>) | undefined | null,
        onrejected?: ((reason: any) => R2 | PromiseLike<R2>) | undefined | null
    ): PromiseLike<R1 | R2> {
        let res: Promise<any>;

        if (this[source] === undefined || this[status] === "closed") {
            res = Promise.resolve(this[result]);
        } else if (this[status] === "erred") {
            res = Promise.reject(this[source]);
        } else if (typeof this[source].then === "function") {
            res = Promise.resolve(this[source]);
        } else if (typeof this[source].next === "function") {
            res = processIterator(this[source]);
        } else {
            res = Promise.resolve(this[source]);
        }

        this[status] = "closed";

        return res
            .then(value => (this[result] = value))
            .then(onfulfilled, onrejected);
    }


    catch<R = never>(
        onrejected?: (reason: any) => R | PromiseLike<R>
    ): Promise<T | R> {
        return Promise.resolve(this).then(null, onrejected);
    }
}

export class ThenableGenerator<T = unknown, TReturn = any, TNext = unknown> extends Thenable<T> implements ThenableGeneratorLike<T, TReturn, TNext> {
    next(...args: [] | [TNext]): IteratorResult<T> {
        const value = args[0];
        let res: IteratorResult<T>;

        if (this[source] === undefined || this[status] === "closed") {
            res = { value: void 0, done: true };
        } else if (this[status] === "erred") {
            return this.throw(this[source]);
        } else if (typeof this[source].next === "function") {
            res = this[source].next(value);
        } else {
            res = { value: this[source], done: true };
        }

        if (res.done === true) {
            this[status] = "closed";
            this[result] = res.value;
        }

        return res;
    }

    return(value?: TReturn): IteratorResult<T> {
        this[status] = "closed";
        this[result] = value;

        if (this[source] && typeof this[source].return === "function") {
            return this[source].return(value);
        } else {
            return { value, done: true };
        }
    }

    throw(err?: any): IteratorResult<T, TReturn> | never {
        this[status] = "closed";

        if (this[source] && typeof this[source].throw === "function") {
            return this[source].throw(err) as never;
        } else {
            throw err;
        }
    }

    [Symbol.iterator]() {
        return this;
    };
}

export class ThenableAsyncGenerator<T = unknown, TReturn = any, TNext = unknown> extends Thenable<T> implements ThenableAsyncGeneratorLike<T, TReturn, TNext> {
    next(...args: [] | [TNext]): Promise<IteratorResult<T, TReturn>> {
        const value = args[0];
        let res: Promise<IteratorResult<any>>;

        if (this[source] === undefined || this[status] === "closed") {
            res = Promise.resolve({ value: void 0, done: true });
        } else if (typeof this[source].next === "function") {
            res = Promise.resolve(this[source].next(value));
        } else {
            res = Promise.resolve(this[source]).then(value => {
                return { value, done: true };
            });
        }

        return res.then(res => {
            if (res.done === true) {
                this[status] = "closed";
                this[result] = res.value;
            }

            return res;
        });
    }

    return(value?: TReturn | PromiseLike<TReturn>): Promise<IteratorResult<T, TReturn>> {
        this[status] = "closed";

        // The input value may be a promise-like object, using Promise.resolve()
        // to guarantee the value is resolved.
        return Promise.resolve(value).then(value => {
            this[result] = value;

            if (this[source] && typeof this[source].return === "function") {
                return Promise.resolve(this[source].return(value));
            } else {
                return Promise.resolve({ value, done: true });
            }
        });
    }

    throw(err?: any): Promise<IteratorResult<T, TReturn> | never> {
        this[status] = "closed";

        if (this[source] && typeof this[source].throw === "function") {
            return Promise.resolve(this[source].throw(err) as never);
        } else {
            return Promise.reject(err);
        }
    }

    [Symbol.asyncIterator]() {
        return this;
    }
}

export const ThenableGeneratorFunction: ThenableGeneratorFunctionConstructor = (function (this: any, fn: Function) {
    if (!(this instanceof ThenableGeneratorFunction)) {
        return new (<any>ThenableGeneratorFunction)(fn);
    }

    function anonymous(this: any, ...args: any[]) {
        try {
            const source = fn.apply(this, args);

            if (typeof source.then === "function" || isAsyncGenerator(source)) {
                return new ThenableAsyncGenerator(source);
            } else {
                return new ThenableGenerator(source);
            }
        } catch (err) {
            return Object.assign(new ThenableGenerator(err), {
                [status]: "erred"
            });
        }
    }

    // HACK, let the returning function be an instance of
    // ThenableGeneratorFunction.
    anonymous.prototype = ThenableGeneratorFunction;
    anonymous.__proto__ = this;

    return anonymous;
}) as any;

Object.setPrototypeOf(ThenableGeneratorFunction, Function);
Object.setPrototypeOf(ThenableGeneratorFunction.prototype, Function.prototype);

/**
 * Creates a generator that implements the `PromiseLike` interface so that it can
 * be awaited in async contexts.
 */
export function create<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]>(
    fn: (...args: TArgs) => AsyncGenerator<T, TReturn, TNext> | AsyncIterable<T> | Promise<T>
): ThenableAsyncGeneratorFunction<T, TReturn, TNext, TArgs>;
export function create<T = unknown, TReturn = any, TNext = unknown, TArgs extends any[] = any[]>(
    fn: (...args: TArgs) => Generator<T, TReturn, TNext> | Iterable<T> | T
): ThenableGeneratorFunction<T, TReturn, TNext, TArgs>;
export function create(fn: Function) {
    return new ThenableGeneratorFunction(fn);
}

export default create;

ThenableGeneratorFunction.create = create;

function isAsyncGenerator(obj: any) {
    return obj !== null
        && typeof obj === "object"
        && typeof obj.next === "function"
        && typeof obj.return === "function"
        && typeof obj.throw === "function"
        && typeof obj[Symbol.asyncIterator] === "function";
}

function processIterator(iterator: Iterator<any> | AsyncIterator<any>) {
    return new Promise<any>((resolve, reject) => {
        function fulfilled(value: any) {
            try { step(iterator.next(value)); } catch (e) { reject(e); }
        }

        function rejected(value: any) {
            try { step(iterator.throw?.(value)); } catch (e) { reject(e); }
        }

        function step(item: any) {
            Promise.resolve(item).then(result => {
                result.done ? resolve(result.value) : new Promise(resolve => {
                    resolve(result.value);
                }).then(fulfilled, rejected);
            });
        }

        step(iterator.next());
    });
}
