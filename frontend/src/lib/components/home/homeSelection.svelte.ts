/**
 * Multi-selection state for the Home page, shared by the flat list and the tree
 * view. `ItemsList` owns the instance and publishes it through context; each
 * `Item` reads it to render its row's selection affordance, so the state doesn't
 * have to be threaded through `TreeViewRoot`/`TreeView`.
 */
import { getContext, setContext } from 'svelte'
import { SvelteMap } from 'svelte/reactivity'
import { isOwner } from '$lib/utils'
import { effectivePath } from './treeViewUtils'
import type { UserExt } from '$lib/stores'

/** A raw app is not a kind of its own here: the listing returns it as an `app`
 * row carrying `raw_app`, which every action addresses through `BulkItem.rawApp`
 * (only the draft overlay distinguishes the two). */
export type BulkKind = 'script' | 'flow' | 'app'

/** The flattened row facts every bulk action needs, snapshotted at selection
 * time so an action still knows what it is acting on after the list reloads. */
export type BulkItem = {
	key: string
	kind: BulkKind
	/** Storage path — what every API call addresses. */
	path: string
	/** What the row shows: a draft-only item is parked at a generated path but
	 * named by the path typed in the editor. */
	displayPath: string
	summary: string
	canWrite: boolean
	owner: boolean
	archived: boolean
	draftOnly: boolean
	/** The authed user has a draft at this path (the listing only flags their own). */
	isDraft: boolean
	rawApp: boolean
}

type RawItem = {
	type?: string
	path: string
	summary?: string
	canWrite?: boolean
	archived?: boolean
	draft_only?: boolean | null
	draft_path?: string | null
	is_draft?: boolean
	raw_app?: boolean
}

/** Row identity for selection. Deliberately hash-free: archiving or renaming
 * mints a new script hash, and a key that moved would orphan the selection. */
export function bulkKey(item: { type?: string; path: string }): string {
	return `${item.type}/${item.path}`
}

export function toBulkItem(
	item: RawItem,
	user: UserExt | undefined,
	workspace: string | undefined
): BulkItem {
	return {
		key: bulkKey(item),
		kind: (item.type ?? 'script') as BulkKind,
		path: item.path,
		displayPath: effectivePath(item),
		summary: item.summary ?? '',
		canWrite: item.canWrite ?? false,
		owner: isOwner(item.path, user, workspace),
		archived: item.archived ?? false,
		draftOnly: !!item.draft_only,
		isDraft: !!item.is_draft,
		rawApp: !!item.raw_app
	}
}

export class HomeSelection {
	/** The page offers multi-selection at all (never to an operator, and not on
	 * the embedded read-only variants of the list). */
	available = $state(false)
	/** Selection mode is on even with nothing selected yet — entered from the
	 * toolbar, so every row reveals its checkbox before the first pick. */
	private explicit = $state(false)
	private selected = new SvelteMap<string, BulkItem>()
	/** Every rendered selectable row, so a shift-click range can resolve the keys
	 * between the anchor and the clicked row back to items. */
	private registry = new SvelteMap<string, BulkItem>()
	private anchor: string | undefined = undefined

	get active(): boolean {
		return this.available && (this.explicit || this.selected.size > 0)
	}

	get size(): number {
		return this.selected.size
	}

	get items(): BulkItem[] {
		return [...this.selected.values()]
	}

	has(key: string): boolean {
		return this.selected.has(key)
	}

	/** Keys of the rows currently on screen. */
	get renderedKeys(): Set<string> {
		return new Set(this.registry.keys())
	}

	/**
	 * Drop selections whose row was on screen before a reload and is gone after it —
	 * deleted, or moved to a new path and so a new key. A row that was never on
	 * screen is left alone: a selection deliberately survives narrowing the view, so
	 * absence there is not evidence the item is gone.
	 */
	dropVanished(renderedBefore: Set<string>): void {
		for (const key of [...this.selected.keys()]) {
			if (renderedBefore.has(key) && !this.registry.has(key)) this.selected.delete(key)
		}
		if (this.selected.size === 0) this.anchor = undefined
	}

	register(item: BulkItem): void {
		this.registry.set(item.key, item)
		// Refresh the snapshot of an already-selected row too: the per-row menu stays
		// usable during a selection and every action reloads the list, so a row that
		// was archived or lost write access meanwhile must not keep passing the
		// eligibility gates on its stale copy.
		if (this.selected.has(item.key)) this.selected.set(item.key, item)
	}

	unregister(key: string): void {
		this.registry.delete(key)
	}

	enter(): void {
		this.explicit = true
	}

	exit(): void {
		this.explicit = false
		this.selected.clear()
		this.anchor = undefined
	}

	/** Keep only these keys selected (a bulk run leaves its failures ticked so
	 * they can be retried); an empty result leaves selection mode entirely. */
	keepOnly(keys: string[]): void {
		const keep = new Set(keys)
		for (const key of [...this.selected.keys()]) {
			if (!keep.has(key)) this.selected.delete(key)
		}
		if (this.selected.size === 0) this.exit()
		else this.anchor = undefined
	}

	toggle(item: BulkItem, range = false): void {
		this.explicit = true
		if (range && this.anchor != undefined && this.anchor !== item.key) {
			if (this.selectRange(this.anchor, item.key)) return
		}
		if (this.selected.has(item.key)) this.selected.delete(item.key)
		else this.selected.set(item.key, item)
		this.anchor = item.key
	}

	/** Visual order is read back from the DOM: the tree nests rows and pages them
	 * in lazily, so no single array holds the rendered order of both views.
	 * Returns false when either end is off-screen, so the caller falls back to a
	 * plain toggle rather than selecting an arbitrary span. */
	private selectRange(from: string, to: string): boolean {
		const keys = Array.from(document.querySelectorAll<HTMLElement>('[data-row-selection-key]')).map(
			(el) => el.dataset.rowSelectionKey ?? ''
		)
		const a = keys.indexOf(from)
		const b = keys.indexOf(to)
		if (a < 0 || b < 0) return false
		for (const key of keys.slice(Math.min(a, b), Math.max(a, b) + 1)) {
			const it = this.registry.get(key)
			if (it) this.selected.set(key, it)
		}
		this.anchor = to
		return true
	}
}

const HOME_SELECTION_KEY = 'homeSelection'

export function setHomeSelection(selection: HomeSelection): void {
	setContext(HOME_SELECTION_KEY, selection)
}

export function getHomeSelection(): HomeSelection | undefined {
	return getContext<HomeSelection | undefined>(HOME_SELECTION_KEY)
}
