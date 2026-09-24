import { randomUUID } from '$lib/utils/uuid'
import { getLocalSetting, storeLocalSetting } from '$lib/utils'

export type DbManagerTabKind = 'data' | 'diagram' | 'sql'

export type DbManagerDiagramTable = { datatable?: string; schema: string; table: string }

export type DbManagerTab =
	| { id: string; kind: 'data'; schema?: string; table?: string }
	/** `tables` stays undefined until the diagram's first draw, automatic or by hand. */
	| { id: string; kind: 'diagram'; tables?: DbManagerDiagramTable[] }
	| { id: string; kind: 'sql'; code?: string }

export type DbManagerDataTab = Extract<DbManagerTab, { kind: 'data' }>

type StoredTabs = { tabs: DbManagerTab[]; activeId: string }

/**
 * The DB manager's tabs over one database, kept in localStorage under `storageKey`. What a tab
 * shows is stored (its table, the diagram's tables, the query); how it was left — scroll,
 * filters, results — lives only in its mounted view.
 */
export class DbManagerTabs {
	tabs = $state<DbManagerTab[]>([])
	activeId = $state('')
	/** Data tabs by last use, most recent first: where the tree opens a table when the active tab
	 * is not a data tab. */
	#dataRecency = $state<string[]>([])
	private storageKey: string | undefined
	private kinds: DbManagerTabKind[]

	constructor(storageKey: string | undefined, kinds: DbManagerTabKind[]) {
		this.storageKey = storageKey
		this.kinds = kinds
		const stored = this.#load()
		this.tabs = stored?.tabs.length
			? stored.tabs
			: kinds.map((kind) => ({ id: randomUUID(), kind }) as DbManagerTab)
		this.activeId = this.tabs.some((t) => t.id === stored?.activeId)
			? stored!.activeId
			: this.tabs[0].id
		this.#dataRecency = this.tabs.filter((t) => t.kind === 'data').map((t) => t.id)
		this.#touch(this.activeId)
	}

	get active(): DbManagerTab {
		return this.tabs.find((t) => t.id === this.activeId) ?? this.tabs[0]
	}

	/** The data tab the tree's selection belongs to: the active one, else the last used. */
	get currentData(): DbManagerDataTab | undefined {
		const id = this.active.kind === 'data' ? this.active.id : this.#dataRecency[0]
		return this.tabs.find((t): t is DbManagerDataTab => t.id === id && t.kind === 'data')
	}

	activate(id: string) {
		if (!this.tabs.some((t) => t.id === id)) return
		this.activeId = id
		this.#touch(id)
		this.#save()
	}

	add(kind: DbManagerTabKind, init: Partial<DbManagerTab> = {}): DbManagerTab {
		const tab = { ...init, id: randomUUID(), kind } as DbManagerTab
		this.tabs.push(tab)
		this.activate(tab.id)
		return tab
	}

	/** Closing the last tab leaves a fresh data tab: the manager always has somewhere to show rows. */
	close(id: string) {
		const index = this.tabs.findIndex((t) => t.id === id)
		if (index === -1) return
		this.tabs.splice(index, 1)
		this.#dataRecency = this.#dataRecency.filter((d) => d !== id)
		if (!this.tabs.length) {
			this.add('data')
			return
		}
		if (this.activeId === id) this.activate(this.tabs[Math.min(index, this.tabs.length - 1)].id)
		else this.#save()
	}

	/** Moves a tab next to another, on the given side of it. */
	move(id: string, targetId: string, side: 'before' | 'after') {
		if (id === targetId) return
		const from = this.tabs.findIndex((t) => t.id === id)
		if (from === -1) return
		const [tab] = this.tabs.splice(from, 1)
		const target = this.tabs.findIndex((t) => t.id === targetId)
		if (target === -1) {
			this.tabs.splice(from, 0, tab)
			return
		}
		this.tabs.splice(side === 'before' ? target : target + 1, 0, tab)
		this.#save()
	}

	update<T extends DbManagerTab>(id: string, patch: Partial<Omit<T, 'id' | 'kind'>>) {
		const tab = this.tabs.find((t) => t.id === id)
		if (!tab) return
		if (Object.entries(patch).every(([k, v]) => (tab as any)[k] === v)) return
		Object.assign(tab, patch)
		this.#save()
	}

	/** The current data tab, activated; one is opened when none is left. */
	focusData(): DbManagerDataTab {
		const tab = this.currentData ?? (this.add('data') as DbManagerDataTab)
		if (this.activeId !== tab.id) this.activate(tab.id)
		return tab
	}

	/** A table named from outside (the URL): shown in the data tab already on it, else in the
	 * current data tab. */
	openTable(schema: string | undefined, table: string) {
		const open = this.tabs.find(
			(t): t is DbManagerDataTab => t.kind === 'data' && t.table === table && t.schema === schema
		)
		if (open) {
			this.activate(open.id)
			return
		}
		this.update<DbManagerDataTab>(this.focusData().id, { schema, table })
	}

	#touch(id: string) {
		if (this.tabs.find((t) => t.id === id)?.kind !== 'data') return
		this.#dataRecency = [id, ...this.#dataRecency.filter((d) => d !== id)]
	}

	#load(): StoredTabs | undefined {
		if (!this.storageKey) return undefined
		try {
			const parsed = JSON.parse(getLocalSetting(this.storageKey) ?? 'null')
			if (!parsed || !Array.isArray(parsed.tabs)) return undefined
			// A kind this database cannot show, e.g. a diagram saved before it became unsupported.
			const tabs = (parsed.tabs as DbManagerTab[]).filter(
				(t) => t && typeof t.id === 'string' && this.kinds.includes(t.kind)
			)
			return { tabs, activeId: parsed.activeId }
		} catch {
			return undefined
		}
	}

	#save() {
		if (!this.storageKey) return
		try {
			const stored: StoredTabs = { tabs: $state.snapshot(this.tabs), activeId: this.activeId }
			storeLocalSetting(this.storageKey, JSON.stringify(stored))
		} catch {}
	}
}
