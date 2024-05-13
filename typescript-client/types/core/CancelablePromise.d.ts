export declare class CancelError extends Error {
    constructor(message: string);
    get isCancelled(): boolean;
}
export interface OnCancel {
    readonly isResolved: boolean;
    readonly isRejected: boolean;
    readonly isCancelled: boolean;
    (cancelHandler: () => void): void;
}
export declare class CancelablePromise<T> implements Promise<T> {
    private _isResolved;
    private _isRejected;
    private _isCancelled;
    readonly cancelHandlers: (() => void)[];
    readonly promise: Promise<T>;
    private _resolve?;
    private _reject?;
    constructor(executor: (resolve: (value: T | PromiseLike<T>) => void, reject: (reason?: unknown) => void, onCancel: OnCancel) => void);
    get [Symbol.toStringTag](): string;
    then<TResult1 = T, TResult2 = never>(onFulfilled?: ((value: T) => TResult1 | PromiseLike<TResult1>) | null, onRejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null): Promise<TResult1 | TResult2>;
    catch<TResult = never>(onRejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null): Promise<T | TResult>;
    finally(onFinally?: (() => void) | null): Promise<T>;
    cancel(): void;
    get isCancelled(): boolean;
}
