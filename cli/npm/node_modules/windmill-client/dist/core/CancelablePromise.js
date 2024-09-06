"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CancelablePromise = exports.CancelError = void 0;
class CancelError extends Error {
    constructor(message) {
        super(message);
        this.name = 'CancelError';
    }
    get isCancelled() {
        return true;
    }
}
exports.CancelError = CancelError;
class CancelablePromise {
    constructor(executor) {
        this._isResolved = false;
        this._isRejected = false;
        this._isCancelled = false;
        this.cancelHandlers = [];
        this.promise = new Promise((resolve, reject) => {
            this._resolve = resolve;
            this._reject = reject;
            const onResolve = (value) => {
                if (this._isResolved || this._isRejected || this._isCancelled) {
                    return;
                }
                this._isResolved = true;
                if (this._resolve)
                    this._resolve(value);
            };
            const onReject = (reason) => {
                if (this._isResolved || this._isRejected || this._isCancelled) {
                    return;
                }
                this._isRejected = true;
                if (this._reject)
                    this._reject(reason);
            };
            const onCancel = (cancelHandler) => {
                if (this._isResolved || this._isRejected || this._isCancelled) {
                    return;
                }
                this.cancelHandlers.push(cancelHandler);
            };
            Object.defineProperty(onCancel, 'isResolved', {
                get: () => this._isResolved,
            });
            Object.defineProperty(onCancel, 'isRejected', {
                get: () => this._isRejected,
            });
            Object.defineProperty(onCancel, 'isCancelled', {
                get: () => this._isCancelled,
            });
            return executor(onResolve, onReject, onCancel);
        });
    }
    get [Symbol.toStringTag]() {
        return "Cancellable Promise";
    }
    then(onFulfilled, onRejected) {
        return this.promise.then(onFulfilled, onRejected);
    }
    catch(onRejected) {
        return this.promise.catch(onRejected);
    }
    finally(onFinally) {
        return this.promise.finally(onFinally);
    }
    cancel() {
        if (this._isResolved || this._isRejected || this._isCancelled) {
            return;
        }
        this._isCancelled = true;
        if (this.cancelHandlers.length) {
            try {
                for (const cancelHandler of this.cancelHandlers) {
                    cancelHandler();
                }
            }
            catch (error) {
                console.warn('Cancellation threw an error', error);
                return;
            }
        }
        this.cancelHandlers.length = 0;
        if (this._reject)
            this._reject(new CancelError('Request aborted'));
    }
    get isCancelled() {
        return this._isCancelled;
    }
}
exports.CancelablePromise = CancelablePromise;
