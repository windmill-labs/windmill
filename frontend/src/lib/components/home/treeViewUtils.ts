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

export type UserItem = {
	username: string
	items: (ItemType | FolderItem)[]
}

function insertItemInFolder(
	root: (ItemType | FolderItem | UserItem)[],
	item: ItemType,
	path: string[]
) {
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

export function groupItems(items: ItemType[] | undefined): (ItemType | FolderItem | UserItem)[] {
	if (!items) {
		return []
	}

	const root: (ItemType | FolderItem | UserItem)[] = []

	items.forEach((item) => {
		const pathSplit = item.path.split('/')
		if (pathSplit[0] === 'u') {
			const username = pathSplit[1]
			let userItem = root.find((f): f is UserItem => 'username' in f && f.username === username) as
				| UserItem
				| undefined

			if (!userItem) {
				userItem = { username, items: [] }
				root.push(userItem)
			}

			if (pathSplit.length > 2) {
				insertItemInFolder(userItem.items, item, pathSplit.slice(2))
			} else {
				userItem.items.push(item)
			}
		} else if (pathSplit[0] === 'f') {
			insertItemInFolder(root, item, pathSplit.slice(1))
		}
	})

	root.sort((a, b) => {
		if ('username' in a && 'folderName' in b) {
			return -1
		}
		if ('folderName' in a && 'username' in b) {
			return 1
		}
		return (a['username'] ?? a['folderName']).localeCompare(b['username'] ?? b['folderName'])
	})

	sortGroup(root)

	return root
}

function sortGroup(group: (ItemType | FolderItem | UserItem)[]) {
	group.forEach((item) => {
		if ('items' in item) {
			item.items.sort((a, b) => {
				if ('folderName' in a && 'folderName' in b) {
					return a.folderName.localeCompare(b.folderName)
				}
				if ('folderName' in a) {
					return -1
				}
				if ('folderName' in b) {
					return 1
				}
				if (isItemType(a) && isItemType(b)) {
					if (a.starred && !b.starred) return -1
					if (!a.starred && b.starred) return 1
					return getModifiedAt(b) - getModifiedAt(a)
				}
				return 0
			})

			sortGroup(item.items)
		}
	})
}

function isItemType(item: ItemType | FolderItem | UserItem): item is ItemType {
	return 'type' in item
}

function getModifiedAt(item: ItemType): number {
	if (item.type === 'app') {
		return new Date(item.edited_at).getTime() || 0
	} else if (item.type === 'script') {
		return new Date(item.created_at).getTime() || 0
	} else if (item.type === 'flow') {
		return new Date(item.edited_at).getTime() || 0
	} else if (item.type === 'raw_app') {
		return new Date(item.edited_at).getTime() || 0
	}

	return 0
}
