import { setContext, getContext } from 'svelte'
import type { Pane, PanesLayout } from './types'

const KEY = 'splitPanesLayout'

export class SplitPanesLayout {
	#layout: Record<string, PanesLayout> = $state({})
	#changeCb: (() => void) | undefined = undefined

	constructor(initialPaneLayouts?: Record<string, PanesLayout>, changeCb?: () => void) {
		this.#layout = initialPaneLayouts ?? {}
		this.#changeCb = changeCb
	}

	get layout() {
		return this.#layout
	}

	set layout(layout: Record<string, PanesLayout>) {
		this.#layout = layout
		this.#changeCb?.()
	}

	setActivePane(layoutId: string, paneIndex: number) {
		if (!this.#layout[layoutId][paneIndex].size) {
			return
		} else {
			this.#layout[layoutId][paneIndex].active = true
			this.#scalePanes(layoutId, paneIndex)
		}
		this.#changeCb?.()
	}

	setPanes(layoutId: string, panes: Pane[]) {
		if (!this.#layout[layoutId]) {
			this.#layout[layoutId] = []
		}

		this.#layout[layoutId] = panes
		this.#scalePanesTo100(layoutId)
		this.#changeCb?.()
	}

	#scalePanes(layoutId: string, paneIndex: number) {
		// Scale other active panes while keeping the current pane size
		const otherPanesTotal = this.#layout[layoutId]
			.filter((pane, idx) => idx !== paneIndex && pane.active)
			.reduce((sum, pane) => sum + (pane.size ?? 0), 0)

		if (otherPanesTotal > 0) {
			const scale = (100 - (this.#layout[layoutId][paneIndex].size ?? 0)) / otherPanesTotal
			this.#layout[layoutId] = this.#layout[layoutId].map((pane, idx) =>
				idx === paneIndex ? pane : { ...pane, size: (pane.size ?? 0) * scale }
			)
		}
	}

	#scalePanesTo100(layoutId: string) {
		const activePanesTotal = this.#layout[layoutId]
			.filter((pane) => pane.active)
			.reduce((sum, pane) => sum + (pane.size ?? 0), 0)

		if (activePanesTotal > 0) {
			const scale = 100 / activePanesTotal
			this.#layout[layoutId] = this.#layout[layoutId].map((pane) =>
				pane.active ? { ...pane, size: (pane.size ?? 0) * scale } : pane
			)
		}
	}

	async removePane(layoutId: string, paneIndex: number) {
		if (this.#layout[layoutId][paneIndex].active) {
			this.#layout[layoutId][paneIndex].active = false
			this.#scalePanesTo100(layoutId)
		}
		this.#changeCb?.()
	}

	handleResize(layoutId: string, newSizes: { size: number; index: number }[]) {
		if (this.#layout[layoutId]) {
			this.#layout[layoutId] = this.#layout[layoutId].map((pane, idx) => {
				const newSize = newSizes.find((s) => s.index === idx)
				return newSize ? { ...pane, size: newSize.size } : pane
			})
		}
		this.#changeCb?.()
	}

	setChangeCb(cb: () => void) {
		this.#changeCb = cb
	}

	getLayoutSnapshot(): Record<string, PanesLayout> {
		return $state.snapshot(this.#layout)
	}
}

export function getSplitPanesLayout(): SplitPanesLayout | undefined {
	return getContext<SplitPanesLayout>(KEY)
}

export function setSplitPanesLayoutContext(layout: SplitPanesLayout) {
	setContext<SplitPanesLayout>(KEY, layout)
}
