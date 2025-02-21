import type { InputConnectionEval } from './inputType'
import { writable, type Writable } from 'svelte/store'
import { deepEqual } from 'fast-equals'
import sum from 'hash-sum'
export interface Subscriber<T> {
	id?: string
	next(v: T, force?: boolean): void
}

export interface Observable<T> {
	subscribe(x: Subscriber<T>, previousValue: T): () => void
}
export interface Output<T> extends Observable<T> {
	set(x: T, force?: boolean): void
	peak(): T | any | undefined
}

export interface Input<T> extends Subscriber<T> {
	peak(): T | any | undefined
}

export type World = {
	outputsById: Record<string, Record<string, Output<any>>>
	connect: <T>(
		connection: InputConnectionEval,
		next: (x: T, force?: boolean) => void,
		id: string,
		previousValue: any
	) => Input<T>
	stateId: Writable<number>
	newOutput: <T>(id: string, name: string, previousValue: T) => Output<T>
}

export function buildWorld(context: Record<string, any>): Writable<World> {
	const newWorld = buildObservableWorld()
	const stateId = writable(0)
	let writableWorld: Writable<World> | undefined = undefined
	const outputsById: Record<string, Record<string, Output<any>>> = {
		ctx: Object.fromEntries(
			Object.entries(context).map(([k, v]) => {
				return [k, newWorld.newOutput('ctx', k, stateId, v)]
			})
		),
		state: {}
	}

	function newOutput<T>(id: string, name: string, previousValue: T) {
		if (outputsById[id]?.[name]) {
			return outputsById[id][name]
		}
		let o = newWorld.newOutput(id, name, stateId, previousValue)
		if (!outputsById[id]) {
			outputsById[id] = {}
		}
		outputsById[id][name] = o
		writableWorld?.update((x) => x)
		return o
	}
	stateId.update((x) => x + 1)
	writableWorld = writable({
		outputsById,
		connect: newWorld.connect,
		stateId,
		newOutput
	})
	return writableWorld
}

export function buildObservableWorld() {
	const observables: Record<string, Output<any>> = {}

	function connect<T>(
		connection: InputConnectionEval,
		next: (x: T) => void,
		id: string,
		previousValue: T
	): Input<T> {
		if (!connection) {
			return {
				id,
				peak: () => undefined,
				next: () => { }
			}
		}

		const input = cachedInput(next, id)

		const { componentId, id: idc } = connection

		let obs = observables[`${componentId}.${idc}`]

		if (!obs) {
			if (componentId != 'row' && componentId != 'iter') {
				console.warn('Observable at ' + componentId + '.' + idc + ' not found')
			}
			return {
				peak: () => undefined,
				next: () => { }
			}
		}

		obs.subscribe(input, previousValue)
		return input
	}

	function newOutput<T>(
		id: string,
		name: string,
		state: Writable<number>,
		previousValue: T
	): Output<T> {
		const output = settableOutput<T>(state, previousValue)
		observables[`${id}.${name}`] = output
		return output
	}

	return {
		connect,
		newOutput
	}
}
export function cachedInput<T>(nextParam: (x: T, force?: boolean) => void, id?: string): Input<T> {
	let value: T | undefined = undefined

	function peak(): T | undefined {
		return value
	}

	function next(x: T, force?: boolean): void {
		value = x
		nextParam(x, force)
	}

	return {
		id,
		peak,
		next
	}
}

export function settableOutput<T>(state: Writable<number>, previousValue: T): Output<T> {
	let value: T | undefined = previousValue
	const subscribers: Subscriber<T>[] = []

	function subscribe(x: Subscriber<T>, npreviousValue: any) {
		let currentSubscriber = subscribers.findIndex((y) => y === x || (y.id && y.id === x.id))
		if (currentSubscriber == -1) {
			subscribers.push(x)
		} else {
			subscribers[currentSubscriber] = x
		}

		// Send the current value to the new subscriber if it already exists
		if (value !== undefined && !deepEqual(value, npreviousValue)) {
			x.next(value, false)
		}

		// return a callback to unsubscribe
		return () => {
			const index = subscribers.findIndex((y) => y === x || (y.id && y.id === x.id))
			if (index !== -1) {
				subscribers.splice(index, 1)
			}
		}
	}

	let lastHash: any = undefined
	function set(x: T, force: boolean = false) {
		let newHash = typeof x === 'object' ? sum(x) : x
		if (lastHash != newHash || force) {
			lastHash = newHash
			state.update((x) => x + 1)

			if (typeof x === 'object') {
				if (Array.isArray(x)) {
					value = [...x] as any
				} else {
					value = Object.assign({}, x)
				}
			} else {
				value = x
			}
			subscribers.forEach((x) => x.next(value!, force))
		}
	}

	function peak(): T | undefined {
		return value
	}

	return {
		subscribe,
		set,
		peak
	}
}
