/**
 * Starts loading a component's chunk when the calling module is evaluated and exposes it once
 * loaded. Call it from a `<script module>` so a mount after the load renders the component in its
 * first frame: an `{#await import()}` shows its pending branch for a tick even on a cached chunk.
 * A failed load is left unhandled on purpose: the root layout reports chunk-load failures.
 */
export function preloadComponent<T>(load: () => Promise<{ default: T }>): {
	readonly current: T | undefined
} {
	let current = $state.raw<T | undefined>(undefined)
	load().then((m) => {
		current = m.default
	})
	return {
		get current() {
			return current
		}
	}
}
