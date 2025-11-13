import { CancelablePromise } from './gen'

export namespace CancelablePromiseUtils {
	export function then<T, U>(
		promise: CancelablePromise<T>,
		f: (value: T) => CancelablePromise<U>
	): CancelablePromise<U> {
		let promiseToBeCanceled: CancelablePromise<any> = promise
		let p = new CancelablePromise<U>((resolve, reject) => {
			promise
				.then((value1) => {
					let promise2 = f(value1)
					promiseToBeCanceled = promise2
					promise2.then((value2) => resolve(value2)).catch((err) => reject(err))
				})
				.catch((err) => reject(err))
		})
		p.cancel = () => promiseToBeCanceled.cancel()
		return p
	}

	export function pure<T>(value: T): CancelablePromise<T> {
		return new CancelablePromise((resolve) => resolve(value))
	}

	export function err<T>(error: any): CancelablePromise<T> {
		return new CancelablePromise((_, reject) => reject(error))
	}

	export function map<T, U>(
		promise: CancelablePromise<T>,
		f: (value: T) => U
	): CancelablePromise<U> {
		return then(promise, (value) => pure(f(value)))
	}

	export function pipe<T>(
		promise: CancelablePromise<T>,
		f: (value: T) => void
	): CancelablePromise<T> {
		promise.then((value) => {
			f(value)
		})
		return promise
	}

	export function catchErr<T, U>(
		promise: CancelablePromise<T>,
		f: (error: any) => CancelablePromise<U>
	): CancelablePromise<T | U> {
		let promiseToBeCanceled: CancelablePromise<any> = promise
		let p = new CancelablePromise<T | U>((resolve, reject) => {
			promise
				.then((value) => resolve(value))
				.catch((err) => {
					let promise2 = f(err)
					promiseToBeCanceled = promise2
					return promise2.then((value2) => resolve(value2)).catch((err2) => reject(err2))
				})
				.catch((err) => reject(err))
		})
		p.cancel = () => promiseToBeCanceled.cancel()
		return p
	}

	export function finallyDo<T>(promise: CancelablePromise<T>, f: () => void): CancelablePromise<T> {
		promise = map(promise, (value) => (f(), value))
		promise = catchErr(promise, (e) => (f(), err(e)))
		return promise
	}

	// Calls onTimeout if the promise does not settle within timeoutMs milliseconds
	export function onTimeout<T>(
		promise: CancelablePromise<T>,
		timeoutMs: number,
		onTimeout: () => void
	): CancelablePromise<T> {
		let timeoutId: number | undefined = setTimeout(onTimeout, timeoutMs)
		promise = finallyDo(promise, () => {
			if (timeoutId !== undefined) clearTimeout(timeoutId)
		})
		return promise
	}
}
