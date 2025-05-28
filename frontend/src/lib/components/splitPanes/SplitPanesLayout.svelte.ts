import { setContext, getContext, tick } from 'svelte'

const KEY = 'splitPanesLayout'

export type Pane = {
	size: number | undefined
	active: boolean
}

export type PanesLayout = Pane[]

export class SplitPanesLayout {
	#layout: Record<string, PanesLayout> = $state({})
	#changeCb: (() => void) | undefined = undefined
	#readyPanes: Record<string, boolean> = $state({})

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

	mountPane(layoutId: string, paneIndex: number, defaultSize: number) {
		if (!this.#layout[layoutId]) {
			this.#layout[layoutId] = [{ size: defaultSize, active: true }]
		} else if (this.#readyPanes[layoutId]) {
			// Ensure array is long enough
			while (this.#layout[layoutId].length <= paneIndex) {
				this.#layout[layoutId].push({ size: undefined, active: false })
			}
			this.#layout[layoutId][paneIndex].active = true
			this.#layout[layoutId][paneIndex].size = this.#layout[layoutId][paneIndex].size ?? defaultSize
			this.#scalePanes(layoutId, paneIndex)
		} else {
			// Ensure array is long enough
			while (this.#layout[layoutId].length <= paneIndex) {
				this.#layout[layoutId].push({ size: undefined, active: false })
			}

			const currentPane = this.#layout[layoutId][paneIndex]
			currentPane.active = true
			currentPane.size = currentPane.size ?? defaultSize
		}
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

	async unmountPane(layoutId: string, paneIndex: number) {
		// Wait for the dom refresh to avoid update in the case the splitpane has been removed
		await tick()
		if (this.#readyPanes[layoutId]) {
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

	handleSplitPaneReady(layoutId: string) {
		this.#readyPanes[layoutId] = true
		this.#scalePanesTo100(layoutId)
		this.#changeCb?.()
	}

	handleSplitPaneDestroy(layoutId: string) {
		this.#readyPanes[layoutId] = false
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
