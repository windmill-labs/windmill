import type { Item } from '$lib/utils'

/** The homepage's ellipsis menu, which several homepage parts fill without owning it. */
export class HomeHeaderMenu {
	#sources = $state<Record<string, () => Item[]>>({})
	#pulseTimer: ReturnType<typeof setTimeout> | undefined
	pulsing = $state(false)

	items = $derived(Object.values(this.#sources).flatMap((items) => items()))

	/** Returns the unregister function, for `onDestroy`. */
	register(key: string, items: () => Item[]): () => void {
		this.#sources[key] = items
		return () => {
			delete this.#sources[key]
		}
	}

	/** Flashes the ellipsis, for a control that just moved into the menu.
	 *
	 *  A keyframe animation mounted only while pulsing, not a transition: the ellipsis can mount in
	 *  the same flush that calls this, so the frame that would paint it at rest is never drawn and
	 *  a transition would go straight to invisible. Remounting replays it on every call. */
	pulse(ms: number) {
		clearTimeout(this.#pulseTimer)
		this.pulsing = true
		this.#pulseTimer = setTimeout(() => (this.pulsing = false), ms)
	}
}
