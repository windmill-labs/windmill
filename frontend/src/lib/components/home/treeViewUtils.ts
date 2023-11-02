import type { ListableApp, Script, Flow, ListableRawApp } from '$lib/gen'
type TableItem<T, U extends 'script' | 'flow' | 'app' | 'raw_app'> = T & {
	canWrite: boolean
	marked?: string
	type?: U
	time?: number
	starred?: boolean
	has_draft?: boolean
}

type TableScript = TableItem<Script, 'script'>
type TableFlow = TableItem<Flow, 'flow'>
type TableApp = TableItem<ListableApp, 'app'>
type TableRawApp = TableItem<ListableRawApp, 'raw_app'>

export type ItemType = TableScript | TableFlow | TableApp | TableRawApp

export interface FolderItem {
	folderName: string
	items: (ItemType | FolderItem)[]
}

function insertItemInFolder(root: (ItemType | FolderItem)[], item: ItemType, path: string[]) {
	let currentLevel = root

	path.forEach((folderName, index) => {
		if (index === path.length - 1) {
			currentLevel.push(item)
		} else {
			let folder = currentLevel.find((f) => 'folderName' in f && f.folderName === folderName) as
				| FolderItem
				| undefined

			if (!folder) {
				folder = { folderName: folderName, items: [] }
				currentLevel.push(folder)
			}
			currentLevel = folder.items
		}
	})
}

export function groupItems(items: ItemType[] | undefined): (ItemType | FolderItem)[] {
	if (!items) {
		return []
	}

	const root: (ItemType | FolderItem)[] = []

	items.forEach((item) => {
		const pathSplit = item.path.split('/')
		if (pathSplit[0] === 'u') {
			root.push(item)
		} else if (pathSplit[0] === 'f') {
			insertItemInFolder(root, item, pathSplit.slice(1))
		}
	})

	return root
}
