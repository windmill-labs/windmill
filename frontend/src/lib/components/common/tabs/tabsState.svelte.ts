import { getContext, setContext } from 'svelte'

const KEY = 'tabsState'

export class TabsState {
	#selected: Record<string, string> = $state({})
	#changeCb: (() => void) | undefined = undefined

	constructor(selected: Record<string, string>, changeCb?: () => void) {
		this.#selected = selected
		this.#changeCb = changeCb
	}

	get selected() {
		return this.#selected
	}

	set selected(value: Record<string, string>) {
		this.#selected = value
		this.#changeCb?.()
	}

	setChangeCb(cb: () => void) {
		this.#changeCb = cb
	}

	getSelected(id: string) {
		return this.#selected[id]
	}

	setSelected(id: string, value: string) {
		this.#selected[id] = value
		this.#changeCb?.()
	}

	getTabsStateSnapshot() {
		return $state.snapshot(this.#selected)
	}
}

export function setTabStateContext(tabsState: TabsState) {
	setContext<TabsState>(KEY, tabsState)
}

export function getTabStateContext(): TabsState | undefined {
	return getContext<TabsState>(KEY)
}
