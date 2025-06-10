// https://github.com/sveltejs/svelte/issues/14600

import type { StateStore } from './utils'

export function withProps<Component, Props>(component: Component, props: Props) {
	const ret = $state({
		component,
		props
	})
	return ret
}

export function stateSnapshot<T>(state: T) {
	return $state.snapshot(state)
}
export function refreshStateStore<T>(store: StateStore<T>): void {
	store.val = $state.snapshot(store.val) as any
}
