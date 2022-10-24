import { writable, type Readable } from 'svelte/store'

export interface StateMachine<T extends readonly string[]> {
	states: T
	currentState: T[number]
}

export interface StateMachineTransition<T extends readonly string[]> {
	from?: Partial<Record<T[number], TransitionFromFunction<T>>>
	to?: Partial<Record<T[number], TransitionToFunction<T>>>
}

type StateMachineInfo<T extends readonly string[]> = {
	states: StateMachine<T>['states']
	currentState: StateMachine<T>['currentState']
}

/** The return value should be a `state` that is available on the current machine. */
export type TransitionFromFunction<T extends readonly string[]> = (
	info: StateMachineInfo<T> & { desiredState: T[number] }
) => T[number]

/** Callback after the state has been changed. */
export type TransitionToFunction<T extends readonly string[]> = (
	info: StateMachineInfo<T> & { previousState: T[number] }
) => T[number]

type StateStore<T extends readonly string[]> = Readable<StateMachine<T>> & {
	setState: (state: T[number]) => StateMachineInfo<T>
}

/** **IMPORTANT:** use the `as const` syntax on the states array to get type safety.
 * *Example: `createStateMachine(['foo', 'bar'] as const)`*
 *
 * Returns a new state machine with the default state set to the first element of the `states` argument. */
export function createStateMachine<T extends readonly string[]>(
	states: T,
	transition: StateMachineTransition<T> = {}
): StateStore<T> {
	const defaultValue: StateMachine<T> = {
		states,
		currentState: states[0]
	}
	const defaultStore = writable(defaultValue)
	const stateStore: StateStore<T> = {
		subscribe: defaultStore.subscribe,
		setState: (nextState) => {
			defaultStore.update((prev) => {
				const previousState = prev.currentState
				const beforeFunc = transition?.from && transition.from[previousState]
				const afterFunc = transition?.to && transition.to[nextState]
				let returnState = nextState

				if (beforeFunc) {
					returnState = beforeFunc({
						states,
						currentState: previousState,
						desiredState: nextState
					})
				}
				if (afterFunc) {
					returnState = afterFunc({
						states,
						currentState: returnState,
						previousState
					})
				}

				prev.currentState = returnState
				return prev
			})
			return { states, currentState: nextState }
		}
	}

	return stateStore
}
