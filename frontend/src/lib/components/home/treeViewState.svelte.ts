import { getLocalSetting, storeLocalSetting } from '$lib/utils'

/** A list's tree-view toggle, persisted under `settingName`, and its expand/collapse-all state. */
export class TreeViewState {
	#settingName: string
	#treeView = $state(false)
	collapseAll = $state(true)

	constructor(settingName: string) {
		this.#settingName = settingName
		this.#treeView = getLocalSetting(settingName) == 'true'
	}

	get treeView() {
		return this.#treeView
	}

	set treeView(value: boolean) {
		this.#treeView = value
		storeLocalSetting(this.#settingName, value ? 'true' : undefined)
	}
}

/** The tree nodes a user opened by hand, by full path prefix, persisted under `settingName`. */
export class TreeOpenMemory {
	// Bounds what a long-lived browser accumulates; the oldest openings drop first.
	static readonly MAX = 200
	#settingName: string
	#open: string[]

	constructor(settingName: string) {
		this.#settingName = settingName
		let stored: unknown
		try {
			stored = JSON.parse(getLocalSetting(settingName) ?? '[]')
		} catch {}
		this.#open = Array.isArray(stored) ? stored.filter((k) => typeof k === 'string') : []
	}

	isOpen(prefix: string): boolean {
		return this.#open.includes(prefix)
	}

	setOpen(prefix: string, open: boolean) {
		this.#open = this.#open.filter((k) => k !== prefix)
		if (open) this.#open = [...this.#open, prefix].slice(-TreeOpenMemory.MAX)
		this.#save()
	}

	clear() {
		this.#open = []
		this.#save()
	}

	#save() {
		storeLocalSetting(this.#settingName, this.#open.length ? JSON.stringify(this.#open) : undefined)
	}
}
