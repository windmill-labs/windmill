declare namespace svelteHTML {
	interface HTMLAttributes<T> {
		'on:click_outside'?: (event: CustomEvent) => void
		'on:pointerdown_outside'?: (event: CustomEvent) => void
		'on:pointerdown_connecting'?: (event: CustomEvent) => void
	}
}
