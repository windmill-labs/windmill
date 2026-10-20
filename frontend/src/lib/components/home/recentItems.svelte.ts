import { onUserChange, scopedKey } from '$lib/userScopedStorage'

export type RecentKind = 'script' | 'flow' | 'app' | 'raw_app' | 'agent'

export type RecentItem = {
	kind: RecentKind
	path: string
	summary?: string
	/** Epoch ms of the last time the item was opened. */
	at: number
}

const MAX_ITEMS = 12

// Per user (scopedKey) and per workspace: what one person opened in one workspace says nothing
// about another, and a shared browser must not show the previous user's items.
function storageKey(workspace: string): string | undefined {
	return scopedKey(`homeRecentItems:${workspace}`)
}

function read(workspace: string): RecentItem[] {
	const key = storageKey(workspace)
	if (!key) return []
	try {
		const parsed = JSON.parse(localStorage.getItem(key) ?? '[]')
		return Array.isArray(parsed)
			? parsed.filter((x) => x && typeof x.path === 'string' && typeof x.kind === 'string')
			: []
	} catch {
		return []
	}
}

function write(workspace: string, items: RecentItem[]): void {
	const key = storageKey(workspace)
	if (!key) return
	try {
		localStorage.setItem(key, JSON.stringify(items))
	} catch {}
}

class RecentItems {
	// Bumped on every write and on the user resolving, so readers re-read storage.
	#version = $state(0)
	// A page opened by a cold load can finish loading before the user's email resolves, and the
	// storage key needs it; those visits wait here instead of being dropped.
	#pending: { workspace: string; item: Omit<RecentItem, 'at'> }[] = []

	constructor() {
		onUserChange((email) => {
			if (email) {
				const pending = this.#pending
				this.#pending = []
				for (const p of pending) this.record(p.workspace, p.item)
			}
			this.#version++
		})
	}

	list(workspace: string): RecentItem[] {
		this.#version
		return read(workspace)
	}

	/** Records an item as just opened. A missing summary keeps the one already stored. */
	record(workspace: string, item: Omit<RecentItem, 'at'>): void {
		if (!workspace || !item.path) return
		if (!storageKey(workspace)) {
			this.#pending.push({ workspace, item })
			return
		}
		const previous = read(workspace)
		const existing = previous.find((x) => x.kind === item.kind && x.path === item.path)
		const next: RecentItem = {
			...item,
			summary: item.summary || existing?.summary,
			at: Date.now()
		}
		write(workspace, [next, ...previous.filter((x) => x !== existing)].slice(0, MAX_ITEMS))
		this.#version++
	}

	forget(workspace: string, kind: RecentKind, path: string): void {
		write(
			workspace,
			read(workspace).filter((x) => !(x.kind === kind && x.path === path))
		)
		this.#version++
	}
}

export const recentItems = new RecentItems()
