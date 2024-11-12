declare namespace svelte.JSX {
	interface DOMAttributes<T> {
		onclick_outside?: CompositionEventHandler<T>
		onpointerdown_outside?: (event: CustomEvent) => void
	}
}
