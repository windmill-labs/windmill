import { untrack } from 'svelte'
import { ScriptService } from '$lib/gen'
import { cacheIdentity } from '$lib/persistedCache.svelte'
import { readSessionHint, sameSessionSince } from '$lib/lastSession'
import { onUserChange, scopedKey } from '$lib/userScopedStorage'
import { latestRequest } from '$lib/utils/latestRequest'

export type RecentKind = 'script' | 'flow' | 'app' | 'raw_app' | 'agent'

export type RecentItem = {
	kind: RecentKind
	path: string
	summary?: string
	/** Epoch ms of the last time the item was opened, in its editor or on its own page. */
	at: number
	/** How it was last opened: in its editor, or on its page to run or use it. Items stored
	 * before views were recorded have none, and were all edits. */
	via?: RecentVia
	/** Epoch ms of the last time it was renamed in place. A check sent before then must not
	 * drop it: it asked about the path the item has just left. */
	movedAt?: number
}

export type RecentVia = 'edit' | 'view'

type RecordedItem = Omit<RecentItem, 'at' | 'via' | 'movedAt'> & { via: RecentVia }

/** Every app, raw ones included, is one item at its path: apps share one path namespace, and
 * favorites and the app listings name both `app`. */
export function itemKind(kind: RecentKind): Exclude<RecentKind, 'raw_app'> {
	return kind === 'raw_app' ? 'app' : kind
}

export function itemKey(kind: RecentKind, path: string): string {
	return `${itemKind(kind)}/${path}`
}

const MAX_ITEMS = 30
export const MAX_AGE_DAYS = 7
const MAX_AGE_MS = MAX_AGE_DAYS * 24 * 60 * 60 * 1000

function fresh(items: RecentItem[]): RecentItem[] {
	const cutoff = Date.now() - MAX_AGE_MS
	return items.filter((x) => typeof x.at === 'number' && x.at >= cutoff)
}

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
	// `++` reads the counter too: bumped from an effect (a page recording its visit), the effect
	// would depend on it and re-run on its own write, forever.
	#bump() {
		untrack(() => this.#version++)
	}
	// A page opened by a cold load can finish loading before the user's email resolves, and the
	// storage key needs it; those writes wait here instead of being dropped.
	#pending: (() => void)[] = []
	#checks = latestRequest()

	constructor() {
		onUserChange((email) => {
			if (email) {
				const pending = this.#pending
				this.#pending = []
				for (const apply of pending) apply()
			}
			this.#bump()
		})
	}

	#whenUserKnown(workspace: string, apply: () => void): boolean {
		if (storageKey(workspace)) return true
		this.#pending.push(apply)
		return false
	}

	list(workspace: string): RecentItem[] {
		this.#version
		return fresh(read(workspace))
	}

	/** Records an item as just opened. A missing summary keeps the one already stored. */
	record(workspace: string, item: RecordedItem): void {
		if (!workspace || !item.path) return
		if (!this.#whenUserKnown(workspace, () => this.record(workspace, item))) return
		const previous = fresh(read(workspace))
		const key = itemKey(item.kind, item.path)
		const existing = previous.find((x) => itemKey(x.kind, x.path) === key)
		const next: RecentItem = {
			...item,
			summary: item.summary || existing?.summary,
			at: Date.now()
		}
		write(workspace, [next, ...previous.filter((x) => x !== existing)].slice(0, MAX_ITEMS))
		this.#bump()
	}

	/** Moves an item to its new path, where it was: its place and time are the user's visit,
	 * which a rename is not. An entry already at the new path is dropped: the server refuses to
	 * rename onto a live item, so it was a gone one's. */
	rename(
		workspace: string,
		kind: RecentKind,
		fromPath: string,
		toPath: string,
		summary: string
	): void {
		if (!workspace || !fromPath || !toPath) return
		if (
			!this.#whenUserKnown(workspace, () => this.rename(workspace, kind, fromPath, toPath, summary))
		)
			return
		const items = fresh(read(workspace))
		const source = items.find((x) => itemKey(x.kind, x.path) === itemKey(kind, fromPath))
		if (!source) return
		const moved = fromPath !== toPath
		if (!moved && summary === source.summary) return
		const next: RecentItem = {
			...source,
			path: toPath,
			summary,
			...(moved && { movedAt: Date.now() })
		}
		const target = items.find(
			(x) => x !== source && itemKey(x.kind, x.path) === itemKey(kind, toPath)
		)
		write(
			workspace,
			items.filter((x) => x !== target).map((x) => (x === source ? next : x))
		)
		this.#bump()
	}

	forget(workspace: string, kind: RecentKind, path: string): void {
		const key = itemKey(kind, path)
		write(
			workspace,
			read(workspace).filter((x) => itemKey(x.kind, x.path) !== key)
		)
		this.#bump()
	}

	/**
	 * Drops the items the server no longer has at their path, however they went: renamed,
	 * deleted or archived, from this browser or anywhere else. Only what was asked about is
	 * dropped, and only if it was neither opened nor moved since the question was sent. A reply
	 * for another user or session, or one a later check superseded, is ignored, and a failed
	 * check drops nothing.
	 */
	async prune(workspace: string): Promise<void> {
		const identity = cacheIdentity()
		if (!workspace || !identity) return
		// Not `list`: it subscribes, and a caller's effect would re-run on this check's own write.
		const asked = fresh(read(workspace))
		if (asked.length === 0) return
		const isLatest = this.#checks.start()
		const sentWith = readSessionHint()
		const sentAt = Date.now()
		let live: Awaited<ReturnType<typeof ScriptService.listRunnables>>
		try {
			live = await ScriptService.listRunnables({
				workspace,
				paths: [...new Set(asked.map((x) => x.path))].join(','),
				includeWithoutMain: true,
				// Everything absent is dropped, so it must all fit in one page: the listing has
				// at most one row per kind and path, and Recent holds at most MAX_ITEMS paths.
				perPage: 4 * MAX_ITEMS
			})
		} catch (e) {
			console.error('Could not check the recent items', e)
			return
		}
		if (!isLatest() || cacheIdentity() !== identity || !sameSessionSince(sentWith)) return
		const checked = new Set(asked.map((x) => itemKey(x.kind, x.path)))
		const found = new Set(live.items.map((x) => itemKey(x.type, x.path)))
		// Re-read: what changed while the check was out is newer than the snapshot it asked about.
		const items = read(workspace)
		const kept = items.filter((x) => {
			const key = itemKey(x.kind, x.path)
			return !checked.has(key) || found.has(key) || x.at >= sentAt || (x.movedAt ?? 0) >= sentAt
		})
		if (kept.length === items.length) return
		write(workspace, kept)
		this.#bump()
	}
}

export const recentItems = new RecentItems()
