import type { AppInput } from './inputType'
import { writable, type Writable } from 'svelte/store'
import { deepEqual } from 'fast-equals'

export interface Subscriber<T> {
	id?: string
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
	connect: <T>(inputSpec: AppInput, next: (x: T) => void, id?: string) => Input<T>
	stateId: Writable<number>
	newOutput: <T>(id: string, name: string, previousValue: T) => Output<T>
	initializedOutputs: number
	initialized: boolean
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
			writableWorld?.update((x) => x)
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
		newOutput,
		initializedOutputs: 0,
		initialized: false
	})
	return writableWorld
}

export function buildObservableWorld() {
	const observables: Record<string, Output<any>> = {}

	function connect<T>(inputSpec: AppInput, next: (x: T) => void, id?: string): Input<T> {
		if (inputSpec.type === 'static') {
			return {
				id,
				peak: () => inputSpec.value,
				next: () => {}
			}
		} else if (inputSpec.type === 'connected') {
			const connection = inputSpec.connection

			if (!connection) {
				return {
					id,
					peak: () => undefined,
					next: () => {}
				}
			}

			const { componentId, path } = connection
			const input = cachedInput(next, `${componentId}-${path}-${id}`)

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
				id,
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
export function cachedInput<T>(nextParan: (x: T) => void, id?: string): Input<T> {
	let value: T | undefined = undefined

	function peak(): T | undefined {
		return value
	}

	function next(x: T): void {
		value = x
		nextParan(x)
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

	function subscribe(x: Subscriber<T>) {
		let currentSubscriber = subscribers.findIndex((y) => y === x || (y.id && y.id === x.id))
		if (currentSubscriber == -1) {
			subscribers.push(x)
		} else {
			subscribers[currentSubscriber] = x
		}

		// Send the current value to the new subscriber if it already exists
		if (value !== undefined) {
			x.next(value)
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
