import { watch } from 'runed'

/** How long a tool-call status label stays up before a newer one replaces it. */
export const LABEL_MIN_MS = 700

/**
 * A value that, once shown, stays shown for at least `minMs`. A change arriving sooner waits
 * for the rest of that time, and only the latest of the changes made meanwhile is shown. The
 * first value starts the window too, so a change right after mount is held like any other.
 * Must be constructed during component initialisation.
 */
export class HeldValue<T> {
	#current = $state() as T
	#shownAt = Date.now()

	constructor(getter: () => T, minMs: () => number) {
		this.#current = getter()
		watch(
			getter,
			(value) => {
				const wait = this.#shownAt + minMs() - Date.now()
				if (wait <= 0) {
					this.#show(value)
					return
				}
				// Re-run on every change, so the cleanup drops the pending value in favour of the
				// newer one, and also clears the timer on unmount.
				const timer = setTimeout(() => this.#show(value), wait)
				return () => clearTimeout(timer)
			},
			{ lazy: true }
		)
	}

	#show(value: T) {
		this.#current = value
		this.#shownAt = Date.now()
	}

	get current(): T {
		return this.#current
	}
}
