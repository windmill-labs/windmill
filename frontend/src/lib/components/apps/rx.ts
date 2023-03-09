import type { AppInput } from './inputType'
import { writable, type Writable } from 'svelte/store'
import { deepEqual } from 'fast-equals'

export interface Subscriber<T> {
	next(v: T)
}

export interface Observable<T> {
	subscribe(x: Subscriber<T>)
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
	connect: <T>(inputSpec: AppInput, next: (x: T) => void) => Input<T>
	state: Writable<number>
}

export function buildWorld(
	components: Record<string, string[]>,
	previousWorld: World | undefined,
	context: Record<string, any>
): World {
	const newWorld = buildObservableWorld()
	const state = writable(0)

	const outputsById: Record<string, Record<string, Output<any>>> = {
		ctx: Object.fromEntries(
			Object.entries(context).map(([k, v]) => {
				return [k, newWorld.newOutput('ctx', k, state, v)]
			})
		)
	}
	for (const [k, outputs] of Object.entries(components)) {
		outputsById[k] = {}

		for (const o of outputs) {
			outputsById[k][o] = newWorld.newOutput(
				k,
				o,
				state,
				previousWorld?.outputsById[k]?.[o]?.peak()
			)
		}
	}
	state.update((x) => x + 1)

	return { outputsById, connect: newWorld.connect, state }
}

export function buildObservableWorld() {
	const observables: Record<string, Output<any>> = {}

	function connect<T>(inputSpec: AppInput, next: (x: T) => void): Input<T> {
		if (inputSpec.type === 'static') {
			return {
				peak: () => inputSpec.value,
				next: () => {}
			}
		} else if (inputSpec.type === 'connected') {
			const input = cachedInput(next)

			const connection = inputSpec.connection

			if (!connection) {
				return {
					peak: () => undefined,
					next: () => {}
				}
			}

			const { componentId, path } = connection

			const [p] = path ? path.split('.')[0].split('[') : [undefined]

			let obs = observables[`${componentId}.${p}`]

			if (!obs) {
				console.warn('Observable at ' + componentId + '.' + p + ' not found')
				return {
					peak: () => undefined,
					next: () => {}
				}
			}

			obs.subscribe(input)
			input.next(obs.peak())
			return input
		} else if (inputSpec.type === 'user') {
			return {
				peak: () => inputSpec.value,
				next: () => {}
			}
		} else {
			throw Error('Unknown input type ' + inputSpec)
		}
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
export function cachedInput<T>(nextParan: (x: T) => void): Input<T> {
	let value: T | undefined = undefined

	function peak(): T | undefined {
		return value
	}

	function next(x: T): void {
		value = x
		nextParan(x)
	}

	return {
		peak,
		next
	}
}

export function settableOutput<T>(state: Writable<number>, previousValue: T): Output<T> {
	let value: T | undefined = previousValue
	const subscribers: Subscriber<T>[] = []

	function subscribe(x: Subscriber<T>) {
		if (!subscribers.includes(x)) {
			subscribers.push(x)

			// Send the current value to the new subscriber if it already exists
			if (value !== undefined) {
				x.next(value)
			}
		}
	}

	function set(x: T, force: boolean = false) {
		if (!deepEqual(value, x) || force) {
			state.update((x) => x + 1)

			value = x

			subscribers.forEach((x) => x.next(value!))
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
