import { CancelablePromise } from './gen'

export namespace CancelablePromiseUtils {
	export function then<T, U>(
		promise: CancelablePromise<T>,
		f: (value: T) => CancelablePromise<U>
	): CancelablePromise<U> {
		return new CancelablePromise<U>((resolve, reject, onCancel) => {
			let promiseToBeCanceled: CancelablePromise<any> = promise
			onCancel(() => promiseToBeCanceled.cancel())
			promise
				.then((value1) => {
					let promise2 = f(value1)
					promiseToBeCanceled = promise2
					promise2.then((value2) => resolve(value2)).catch((err) => reject(err))
				})
				.catch((err) => reject(err))
		})
	}

	export function pure<T>(value: T): CancelablePromise<T> {
		return new CancelablePromise((resolve) => resolve(value))
	}

	export function pureErr<T>(error: any): CancelablePromise<T> {
		return new CancelablePromise((_, reject) => reject(error))
	}

	export function map<T, U>(
		promise: CancelablePromise<T>,
		f: (value: T) => U
	): CancelablePromise<U> {
		return then(promise, (value) => pure(f(value)))
	}

	export function catchErr<T>(
		promise: CancelablePromise<T>,
		f: (error: any) => CancelablePromise<T>
	): CancelablePromise<T> {
		return new CancelablePromise<T>((resolve, reject, onCancel) => {
			let promiseToBeCanceled: CancelablePromise<any> = promise
			onCancel(() => promiseToBeCanceled.cancel())
			promise
				.then((value) => resolve(value))
				.catch((err) => {
					let promise2 = f(err)
					promiseToBeCanceled = promise2
					promise2.then((value2) => resolve(value2)).catch((err2) => reject(err2))
				})
		})
	}

	export function finallyDo<T>(promise: CancelablePromise<T>, f: () => void): CancelablePromise<T> {
		return new CancelablePromise<T>((resolve, reject, onCancel) => {
			let promiseToBeCanceled: CancelablePromise<any> = promise
			onCancel(() => promiseToBeCanceled.cancel())
			promise
				.then((value) => {
					f()
					resolve(value)
				})
				.catch((err) => {
					f()
					reject(err)
				})
		})
	}
}
