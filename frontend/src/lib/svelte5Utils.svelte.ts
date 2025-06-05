// https://github.com/sveltejs/svelte/issues/14600

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
