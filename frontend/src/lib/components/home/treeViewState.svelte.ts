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
