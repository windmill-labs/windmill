<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { PIPELINE_DRAFT_KIND, pipelineFolderFromBundlePath } from '$lib/pipelinePaths'
	import { Button, Skeleton } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import {
		AssetService,
		FolderService,
		UserService,
		type ListableApp,
		type Script,
		ScriptService,
		type Flow,
		type ListableRawApp,
		type RunnableItem
	} from '$lib/gen'
	import { resource } from 'runed'
	import { getDraftItems } from '$lib/workspaceDrafts.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import type uFuzzy from '@leeoniya/ufuzzy'
	import {
		ArrowDownUp,
		CheckSquare,
		ChevronsDownUp,
		ChevronsUpDown,
		Code2,
		LayoutDashboard,
		Tag
	} from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import CreateActionsMenu from './CreateActionsMenu.svelte'
	import ContentSearchInner from '$lib/components/ContentSearchInner.svelte'
	import type { Item as MenuItem } from '$lib/utils'

	import { HOME_SEARCH_SHOW_FLOW, HOME_SEARCH_PLACEHOLDER } from '$lib/consts'

	import SearchItems from '../SearchItems.svelte'
	import FilterSearchbar, {
		useUrlSyncedFilterInstance,
		type FilterSchemaRec
	} from '$lib/components/FilterSearchbar.svelte'
	import NoItemFound from './NoItemFound.svelte'
	import ListFilters from './ListFilters.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from './FlowIcon.svelte'
	import { canWrite, getLocalSetting, isOwner, storeLocalSetting } from '$lib/utils'
	import { sendUserToast } from '$lib/toast'
	import Drawer from '../common/drawer/Drawer.svelte'
	import HighlightCode from '../HighlightCode.svelte'
	import DrawerContent from '../common/drawer/DrawerContent.svelte'
	import Item from './Item.svelte'
	import TreeViewRoot from './TreeViewRoot.svelte'
	import { effectivePath, type ItemType } from './treeViewUtils'
	import { tick, untrack } from 'svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'
	import { NetworkIcon } from 'lucide-svelte'
	import { base } from '$lib/base'
	import BulkActionsBar from './BulkActionsBar.svelte'
	import { HomeSelection, setHomeSelection, toBulkItem } from './homeSelection.svelte'
	interface Props {
		subtab?: 'flow' | 'script' | 'app'
		showEditButtons?: boolean
	}

	let { subtab = $bindable('script'), showEditButtons = true }: Props = $props()

	// Which user-folder scoping toggle (if any) this role gets. Declared before the
	// FilterSearchbar schema and the derived filters that both read it.
	let filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined = $derived(
		$userStore?.non_member
			? 'only f/*'
			: $userStore?.is_admin || $userStore?.is_super_admin
				? 'u/username and f/*'
				: undefined
	)

	// FilterSearchbar schema — `_default_` is the free-text search; the rest mirror the
	// boolean/kind list filters. Owner and label are also reachable as searchbar presets
	// (searchPresets) and as on-page chip rows (the ListFilters markup below). `content` is
	// a distinct mode: it swaps the list for the client-side content-match view below
	// (usable on any instance, not EE-gated).
	let searchFilterSchema = $derived({
		_default_: { type: 'string' as const, hidden: true },
		content: {
			type: 'string' as const,
			label: 'Content',
			description: 'Search across item contents'
		},
		// Owner (u/<user> or f/<folder>) and label are offered as presets built from what the
		// list actually holds (see searchPresets); owner is a server path-scope, label a
		// client-side filter over the loaded rows.
		owner: { type: 'string' as const, label: 'Owner' },
		label: { type: 'string' as const, label: 'Label' },
		kind: {
			type: 'oneof' as const,
			label: 'Kind',
			options: [
				{ value: 'script', label: 'Script' },
				...(HOME_SEARCH_SHOW_FLOW ? [{ value: 'flow', label: 'Flow' }] : []),
				{ value: 'app', label: 'App' }
			]
		},
		archived: { type: 'boolean' as const, label: 'Only archived' },
		// include_library and only_user_folders are role-dependent, but their KEYS stay unconditional
		// (toggling `hidden` instead): useUrlSyncedFilterInstance snapshots the key set once and Home
		// survives workspace switches, so a key first appearing after a role change would never
		// URL-sync. The searchbar hides the inactive ones.
		include_library: {
			type: 'boolean' as const,
			label: 'Include library scripts',
			// On by default, so selecting it means "turn it off" — keep the picker.
			default: true,
			hidden: !($userStore && !$userStore.operator)
		},
		only_user_folders: {
			type: 'boolean' as const,
			label:
				filterUserFoldersType === 'only f/*'
					? 'Only f/*'
					: `Only u/${$userStore?.username} and f/*`,
			hidden: !filterUserFoldersType
		}
	} satisfies FilterSchemaRec)

	// Legacy Home links stored free-text in `search`, owner scope in `filter`, and could carry
	// `kind=all` — none of which the generic searchbar sync (keys `_default_`, `owner`, and a kind
	// enum without `all`) understands. Rewrite them once, before the sync reads window.location, so
	// shared/bookmarked URLs still restore and an invalid `kind=all` can't wedge later edits.
	if (typeof window !== 'undefined') {
		const url = new URL(window.location.href)
		const p = url.searchParams
		let changed = false
		const legacySearch = p.get('search')
		if (legacySearch !== null) {
			if (!p.has('_default_')) p.set('_default_', legacySearch)
			p.delete('search')
			changed = true
		}
		const legacyOwner = p.get('filter')
		if (legacyOwner !== null) {
			if (!p.has('owner')) p.set('owner', legacyOwner)
			p.delete('filter')
			changed = true
		}
		if (p.get('kind') === 'all') {
			p.delete('kind')
			changed = true
		}
		if (changed) {
			history.replaceState(
				history.state,
				'',
				`${url.pathname}${p.toString() ? `?${p}` : ''}${url.hash}`
			)
		}
	}

	// Single URL-synced source of truth for the searchbar-driven filters.
	let filterValues = useUrlSyncedFilterInstance(untrack(() => searchFilterSchema))

	// Derived views the rest of the data layer reads. The merged-endpoint reload effect
	// depends on these (search, kind, archived, library, user-folder scope), so changing a
	// searchbar chip reloads the server stream exactly as toggling the old controls did.
	let filter = $derived((filterValues.val._default_ ?? '') as string)
	let itemKind = $derived((filterValues.val.kind ?? 'all') as 'script' | 'flow' | 'app' | 'all')
	let archived = $derived(!!filterValues.val.archived)
	let includeWithoutMain = $derived((filterValues.val.include_library ?? true) as boolean)
	let filterUserFolders = $derived(!!filterValues.val.only_user_folders)

	// Content search is a distinct mode: its results come from ContentSearchInner
	// (which carries only path + content), so the row-list filters can't
	// apply to it. When it's active we restrict the searchbar to just the content filter,
	// clear any other filters so they don't linger as ignored chips, and hide the row-list
	// controls (kind toggle, tree view) that no longer drive anything.
	let contentActive = $derived(!!filterValues.val.content)
	let searchbarSchema = $derived(
		contentActive ? { content: searchFilterSchema.content } : searchFilterSchema
	)
	$effect(() => {
		if (!contentActive) return
		untrack(() => {
			for (const k of Object.keys(filterValues.val)) {
				if (k !== 'content') delete (filterValues.val as Record<string, unknown>)[k]
			}
		})
	})

	// Content-filter view: reuse the Ctrl-K "Content" search. It loads its own dataset
	// via `.open()`, then filters client-side by `search`. The component is
	// keyed by workspace in the markup, so a workspace switch remounts it (this `bind:this`
	// then points at the fresh instance and re-runs `open()`); the old instance is discarded,
	// so its late in-flight responses can't overwrite the new workspace's results.
	let contentSearchEl: ContentSearchInner | undefined = $state()
	$effect(() => {
		const el = contentSearchEl
		if (el) untrack(() => el.open())
	})

	type TableItem<T, U extends 'script' | 'flow' | 'app' | 'raw_app'> = T & {
		canWrite: boolean
		marked?: string
		type?: U
		time?: number
		starred?: boolean
		hash?: string
		// Fetch ordinal: the position this row arrived in from the server (which already
		// applied the chosen order + starred-first). Sorting by it reproduces the server's
		// global order EXACTLY — no client re-derivation that could disagree on collation
		// or sub-millisecond ties and make a later page jump above shown rows.
		ord?: number
	}

	type TableScript = TableItem<Script, 'script'>
	type TableFlow = TableItem<Flow, 'flow'>
	type TableApp = TableItem<ListableApp, 'app'>
	type TableRawApp = TableItem<ListableRawApp, 'raw_app'>

	// Folders that are data pipelines, surfaced as their own "Pipeline" entry
	// (the member scripts are folded into it, not listed individually). Two
	// sources: deployed pipelines (folders with ≥1 `auto_kind='pipeline'` script,
	// cheap via the partial index) AND bundle-phase pipelines that only exist as a
	// `data_pipeline` draft so far — so a pipeline shows up the moment its first
	// node is drafted, before anything is deployed.
	let pipelineFoldersRes = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return new Set<string>()
			const folders = new Set<string>()
			try {
				for (const r of await AssetService.listPipelineFolders({ workspace: ws }))
					folders.add(r.folder)
			} catch {
				// Decorative entry — degrade gracefully on failure.
			}
			try {
				for (const d of await getDraftItems(ws)) {
					if (d.kind !== PIPELINE_DRAFT_KIND) continue
					const folder = pipelineFolderFromBundlePath(d.path)
					if (folder) folders.add(folder)
				}
			} catch {
				// Drafts unavailable — show deployed pipelines only.
			}
			return folders
		}
	)
	// Folders of pipeline-member scripts present in the current listing (captured
	// in loadRunnables before they're filtered out). Unioned in so a folder whose
	// only pipeline node is a never-deployed `// pipeline` script draft — not in
	// listPipelineFolders (deployed-only) nor a `data_pipeline` bundle — still gets
	// a pipeline entry instead of vanishing.
	let pipelineMemberFolders = $state(new Set<string>())
	let pipelineFolders = $derived(
		new Set<string>([...(pipelineFoldersRes.current ?? []), ...pipelineMemberFolders])
	)

	// The workspace's full folder list, independent of which items are paged in,
	// so the owner facet and tree show every folder even when its items sit far
	// down the sorted stream. Cheap and cached per workspace.
	let folderNamesRes = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return [] as string[]
			// Page to exhaustion: listFolderNames is capped per page, so a workspace
			// with more folders than the cap would otherwise be truncated.
			const perPage = 1000
			const all: string[] = []
			try {
				for (let page = 1; ; page++) {
					const batch = await FolderService.listFolderNames({ workspace: ws, page, perPage })
					all.push(...batch)
					if (batch.length < perPage) break
				}
			} catch {
				// Best-effort facet; return whatever we gathered.
			}
			return all
		}
	)
	let allFolderOwners = $derived((folderNamesRes.current ?? []).map((f) => `f/${f}`))
	// Every workspace username, so a user whose items sit beyond the loaded browse
	// window is still a selectable owner chip (scoping the stream to `u/<user>/`).
	// Without this, user owners would derive only from loaded rows and a user past
	// the first page would be unreachable in the tree without searching.
	let usernamesRes = resource(
		() => $workspaceStore,
		async (ws) => {
			if (!ws) return [] as string[]
			try {
				return await UserService.listUsernames({ workspace: ws })
			} catch {
				return [] as string[] // best-effort facet
			}
		}
	)
	let allUserOwners = $derived((usernamesRes.current ?? []).map((u) => `u/${u}`))

	let scripts: TableScript[] | undefined = $state()
	let flows: TableFlow[] | undefined = $state()
	let apps: TableApp[] | undefined = $state()
	let raw_apps: TableRawApp[] | undefined = $state()
	// Monotonic fetch-order counter stamped onto each row as it arrives (see TableItem.ord).
	let fetchOrd = 0

	let filteredItems: (TableScript | TableFlow | TableApp | TableRawApp)[] = $state([])

	let loading = $state(true)

	let nbDisplayed = $state(15)

	// Keyset cursor for the next browse page; null once the stream is exhausted.
	let serverCursor: string | undefined = undefined
	let hasMoreServer = $state(false)
	// Guards against out-of-order responses: only the latest request applies.
	let loadGen = 0

	// Sort selector value -> merged-endpoint order params.
	function sortToParams(o: SortOrder): { orderBy: 'updated' | 'name'; orderDesc: boolean } {
		switch (o) {
			case 'updated_asc':
				return { orderBy: 'updated', orderDesc: false }
			case 'name_asc':
				return { orderBy: 'name', orderDesc: false }
			case 'name_desc':
				return { orderBy: 'name', orderDesc: true }
			case 'updated_desc':
			default:
				return { orderBy: 'updated', orderDesc: true }
		}
	}

	function mapRunnable(it: RunnableItem): TableScript | TableFlow | TableApp {
		const base = {
			...it,
			canWrite:
				canWrite(it.path, (it.extra_perms ?? {}) as any, $userStore) &&
				(it.type === 'script' || it.workspace_id == $workspaceStore) &&
				!$userStore?.operator
		}
		// combinedItems reads a script's time from `created_at`; the endpoint's
		// unified `edited_at` holds exactly that for scripts.
		if (it.type === 'script') return { ...base, created_at: it.edited_at } as unknown as TableScript
		return base as unknown as TableFlow | TableApp
	}

	// The merged, server-ordered, keyset-paginated source. `reset` reloads from
	// the first page (order/filter change or workspace switch); otherwise it
	// appends the next page. All three kinds arrive interleaved and are split into
	// the existing per-kind arrays so the downstream pipeline is unchanged.
	async function loadRunnables(reset: boolean): Promise<void> {
		const ws = $workspaceStore
		if (!ws || !$userStore) return
		// Only the very first load shows the skeleton; reorder/filter reloads keep
		// the toolbar and current items visible (they're replaced on arrival) so the
		// sort control itself doesn't flicker out when you use it.
		// An append (load-more) needs a live cursor. If there isn't one — the stream
		// is exhausted, or a reset just cleared it and its page-1 response hasn't
		// landed — bail so a load-more can't append a fresh page-1 onto the old
		// arrays (mixing streams) or clobber the pending reset's generation.
		if (!reset && serverCursor === undefined) return
		if (scripts === undefined) loading = true
		if (reset) {
			serverCursor = undefined
			hasMoreServer = false
			// Note: this resets only the global/flat stream. The lazy tree's own store
			// (treeOwnerItems/ownerLoad/openOwners) is managed separately — a mode change
			// clears it (see the treeKey effect), while an in-place sort/filter change
			// replaces each open owner's rows atomically as they re-load (see
			// loadOwnerItems), so expanded folders don't blank out mid-reorder.
		}
		const { orderBy, orderDesc } = sortToParams(sortOrder)
		const gen = ++loadGen
		// Snapshot the cursor now: a later request must not consume a cursor minted
		// by an order/filter that has since changed.
		const cursor = reset ? undefined : serverCursor
		let res: { items: RunnableItem[]; next_cursor?: string }
		try {
			res = await ScriptService.listRunnables({
				workspace: ws,
				orderBy,
				orderDesc,
				showArchived: archived ? true : undefined,
				includeWithoutMain: includeWithoutMain ? true : undefined,
				kinds: itemKind !== 'all' ? itemKind : undefined,
				// Selecting an owner/folder scopes the paged stream to it server-side,
				// so a folder's full contents load on demand rather than relying on the
				// folder happening to be within the loaded browse window.
				pathStart: ownerFilter ? ownerFilter + '/' : undefined,
				// Your own not-yet-deployed work belongs in the list you browse; the
				// endpoint sorts and pages it with everything else.
				includeDraftOnly: true,
				perPage: 100,
				cursor
			})
		} catch (e: any) {
			if (gen !== loadGen) return
			loading = false
			sendUserToast(`Failed to load items: ${e?.body ?? e?.message ?? e}`, true)
			return
		}
		// A newer request superseded this one (e.g. order changed mid-flight); drop
		// this response so a stale page/cursor can't be mixed with the new order.
		if (gen !== loadGen) return
		serverCursor = res.next_cursor ?? undefined
		hasMoreServer = !!res.next_cursor

		const s: TableScript[] = reset ? [] : [...(scripts ?? [])]
		const f: TableFlow[] = reset ? [] : [...(flows ?? [])]
		const a: TableApp[] = reset ? [] : [...(apps ?? [])]
		const memberFolders = reset ? new Set<string>() : new Set(pipelineMemberFolders)
		if (reset) fetchOrd = 0
		for (const it of res.items ?? []) {
			if (it.type === 'script') {
				// Pipeline-member scripts are folded into their pipeline entry.
				if (it.auto_kind === 'pipeline') {
					const m = effectivePath(it).match(/^f\/([^/]+)\//)
					if (m) memberFolders.add(m[1])
					continue
				}
				s.push({ ...mapRunnable(it), ord: fetchOrd++ } as TableScript)
			} else if (it.type === 'flow') {
				f.push({ ...mapRunnable(it), ord: fetchOrd++ } as TableFlow)
			} else if (it.type === 'app') {
				a.push({ ...mapRunnable(it), ord: fetchOrd++ } as TableApp)
			}
		}
		scripts = s
		flows = f
		apps = a
		raw_apps = []
		pipelineMemberFolders = memberFolders
		loading = false
	}

	function itemKey(x: { type?: string; path: string; hash?: unknown }): string {
		return `${x.type}/${x.path}${x.hash ? '/' + x.hash : ''}`
	}

	// Merge server rows (from the search-augmentation pass) into the loaded arrays,
	// skipping ones already present. Lets a search reach matches outside the loaded
	// browse window without re-fetching the whole list.
	function mergeRunnables(newItems: RunnableItem[]) {
		const have = new Set([...(scripts ?? []), ...(flows ?? []), ...(apps ?? [])].map(itemKey))
		const s = [...(scripts ?? [])]
		const f = [...(flows ?? [])]
		const a = [...(apps ?? [])]
		const memberFolders = new Set(pipelineMemberFolders)
		let changed = false
		for (const it of newItems) {
			if (it.type === 'script' && it.auto_kind === 'pipeline') {
				const m = it.path.match(/^f\/([^/]+)\//)
				if (m) memberFolders.add(m[1])
				continue
			}
			if (have.has(itemKey(it))) continue
			// Appended after the browse window; a higher ord keeps them after it (search
			// itself ranks by relevance, so this ordinal only matters if sort resumes).
			const mapped = { ...mapRunnable(it), ord: fetchOrd++ }
			if (it.type === 'script') s.push(mapped as TableScript)
			else if (it.type === 'flow') f.push(mapped as TableFlow)
			else if (it.type === 'app') a.push(mapped as TableApp)
			changed = true
		}
		if (changed) {
			scripts = s
			flows = f
			apps = a
			pipelineMemberFolders = memberFolders
		}
	}

	// Per-folder lazy loading for tree view, keyed by the full path prefix a node covers —
	// an owner (`f/<name>` / `u/<name>`) or any folder under one — since the listing
	// endpoint scopes on an arbitrary `path_start`. That is what lets a subfolder be
	// completed on its own instead of only by paging its whole owner.
	//
	// Rows for every prefix live in ONE store (`treeOwnerItems`), never in the global
	// browse arrays: those advance by a single `serverCursor`, so out-of-window rows there
	// would make the flat stream non-contiguous and duplicate once pagination reached them.
	//
	// `treeGen` ties every request to the active scope (order/archived/library/kind/
	// workspace); a reset bumps it so an in-flight response from a stale scope is dropped.
	type OwnerLoadState = {
		cursor?: string
		hasMore: boolean
		loading: boolean
		loaded: boolean
		gen: number
	}
	// Rows per request when loading a prefix in the tree. Larger than the flat list's
	// page because a tree row is a single line and a folder is opened to see what it
	// holds — most folders come in whole on the first click.
	const OWNER_PAGE_SIZE = 300
	// How far one "Load more" will page past rows it already has before giving up and
	// leaving the rest to another click (see the catch-up loop in loadOwnerItems).
	const OWNER_CATCH_UP_PAGES = 5
	// Ceiling on the pages one "Load all" issues. It exists so a prefix that never stops
	// handing back a cursor can't spin forever; hitting it leaves `hasMore` set, so the
	// footer stays and another click resumes where this one stopped.
	const OWNER_LOAD_ALL_PAGES = 100
	let ownerLoad = $state<Record<string, OwnerLoadState>>({})
	let treeOwnerItems = $state<ItemType[]>([])
	let treeGen = 0
	// Prefixes currently loaded and on screen. A reload re-fetches only these: ownerLoad
	// also retains collapsed nodes as a cache, so keying reloads off its entries would
	// re-request every folder ever opened in this scope.
	let openOwners = new Set<string>()

	// The endpoint returns a unified `edited_at` per row; combinedItems derives every
	// kind's sort time from it (scripts read it as `created_at`, see mapRunnable), so
	// the owner store mirrors that to keep tree ordering consistent with the list.
	function toTreeItem(it: RunnableItem): ItemType {
		return {
			...mapRunnable(it),
			time: new Date(it.edited_at ?? 0).getTime()
		} as unknown as ItemType
	}

	// `owner` is the full prefix a node covers: `f/<folder>`, `u/<username>`, or any
	// folder under one. `force` re-fetches its first page even if already loaded (a
	// re-sort / re-filter reload uses it to refresh the loaded rows in place); `all`
	// keeps paging until the prefix's stream is exhausted instead of stopping at a page.
	async function loadOwnerItems(
		owner: string,
		more = false,
		opts?: { force?: boolean; all?: boolean }
	): Promise<void> {
		const force = opts?.force ?? false
		const all = opts?.all ?? false
		const ws = $workspaceStore
		if (!ws || !$userStore) return
		// Track the prefix as open first — even a no-op call (re-expanding a cached node)
		// means it's on screen, so later reloads must refresh it.
		openOwners.add(owner)
		const st = ownerLoad[owner]
		// Only a load for the CURRENT generation blocks a new one. A load left in flight
		// by a superseded generation (treeGen bumped on a sort/filter reload) has already
		// been invalidated, so it must not wedge the owner as permanently loading.
		if (st?.loading && st.gen === treeGen) return
		if (!force && (more ? !st?.hasMore : st?.loaded)) return
		const gen = treeGen
		ownerLoad[owner] = {
			cursor: st?.cursor,
			hasMore: st?.hasMore ?? false,
			loading: true,
			loaded: st?.loaded ?? false,
			gen
		}
		const { orderBy, orderDesc } = sortToParams(sortOrder)
		const prefix = `${owner}/`
		// Only a forced refresh replaces this prefix's rows, so a re-sort swaps them without
		// blanking the tree. Everything else merges: a nested prefix inherits rows from an
		// ancestor's pages, and one page of its own can cover fewer of them than are shown —
		// replacing would delete rows on the click meant to add them.
		const replacing = !more && force
		// Merges a page and answers how many rows it actually added. Reads the live store
		// each time rather than a snapshot, so a page landing while another prefix loads
		// doesn't drop that prefix's rows.
		let firstMerge = true
		const mergePage = (items: RunnableItem[] | undefined): number => {
			const current =
				replacing && firstMerge
					? treeOwnerItems.filter((x) => !effectivePath(x).startsWith(prefix))
					: treeOwnerItems
			firstMerge = false
			const have = new Set(current.map(itemKey))
			const merged = [...current]
			let added = 0
			for (const it of items ?? []) {
				// Pipeline-member scripts are folded into their folder's Pipeline entry, so
				// they never render as their own tree leaf (visiblePipelineFolders drives it).
				if (it.type === 'script' && it.auto_kind === 'pipeline') continue
				if (have.has(itemKey(it))) continue
				merged.push({ ...toTreeItem(it), ord: fetchOrd++ })
				added++
			}
			treeOwnerItems = merged
			return added
		}
		let cursor = more ? st?.cursor : undefined
		let nextCursor: string | undefined
		// A nested prefix's own stream restarts at its first row, which an ancestor's pages
		// may already have brought in — that page then adds nothing and the click would read
		// as broken. Keep paging until one adds something or the stream ends, bounded so a
		// single click can't turn into an unbounded fetch loop. "Load all" instead stops
		// only at the end of the stream, so `loading` stays set for the whole run and the
		// footer resolves to an exact count in one click.
		for (let page = 0; page < (all ? OWNER_LOAD_ALL_PAGES : OWNER_CATCH_UP_PAGES); page++) {
			let res: { items: RunnableItem[]; next_cursor?: string }
			try {
				res = await ScriptService.listRunnables({
					workspace: ws,
					orderBy,
					orderDesc,
					showArchived: archived ? true : undefined,
					includeWithoutMain: includeWithoutMain ? true : undefined,
					kinds: itemKind !== 'all' ? itemKind : undefined,
					pathStart: prefix,
					includeDraftOnly: true,
					perPage: OWNER_PAGE_SIZE,
					cursor
				})
			} catch (e: any) {
				if (gen !== treeGen) return
				// Keep the cursor the pages that did land reached, so the next click resumes
				// instead of re-reading pages that now dedup to nothing. `loaded` moves with
				// it: a node still marked unloaded is retried as a first load, which starts
				// from no cursor and throws the saved one away.
				const prev = ownerLoad[owner]
				const advanced = nextCursor != undefined
				ownerLoad[owner] = {
					...prev,
					cursor: nextCursor ?? prev?.cursor,
					hasMore: advanced || (prev?.hasMore ?? false),
					loaded: advanced || (prev?.loaded ?? false),
					loading: false
				}
				sendUserToast(`Failed to load ${owner}: ${e?.body ?? e?.message ?? e}`, true)
				return
			}
			// The scope moved on while this was in flight (order/archive/library/kind/
			// workspace changed and reset the tree); drop the response so stale rows from
			// another scope can't appear under the current one.
			if (gen !== treeGen) return
			const added = mergePage(res.items)
			nextCursor = res.next_cursor
			cursor = nextCursor
			// Collapsing the node is the only way to stop a run that spans many pages;
			// without this it would keep paging a folder that is no longer on screen. What
			// it reached is committed below, so its footer resumes from there.
			if (nextCursor == undefined || !openOwners.has(owner) || (!all && added > 0)) break
		}
		ownerLoad = Object.fromEntries([
			// A replacing load dropped every row under this prefix, so a nested folder that
			// had paged itself is back to whatever this page holds: its load state has to go
			// with its rows. Left behind, a subfolder marked complete would keep an exact
			// count and no "Load more" over rows this response truncated. Reloads re-fetch
			// the ones still open (see reloadItems), which re-establishes their state.
			...Object.entries(ownerLoad).filter(([p]) => !replacing || !p.startsWith(prefix)),
			[owner, { cursor: nextCursor, hasMore: !!nextCursor, loading: false, loaded: true, gen }]
		])
	}

	function collapseOwner(owner: string): void {
		openOwners.delete(owner)
	}
	// The `f/<folder>` / `u/<user>` prefix a path belongs to.
	function ownerOf(path: string): string {
		return path.split('/').slice(0, 2).join('/')
	}
	// Reload the merged list once and re-fetch the owners that are currently expanded so
	// they don't go blank. Used both for row mutations (create/edit/archive/move/share)
	// and for in-place scope changes (sort/archive/library/kind): those keep the tree in
	// lazy mode (treeKey unchanged, so folders stay open like a file explorer), and the
	// re-fetch reloads each open owner's items in the new order/filter. When a mode
	// change (owner/search) has switched the tree out of lazy mode, there's nothing to
	// re-fetch — the tree remounts and groups the global stream instead.
	async function reloadItems(): Promise<void> {
		// Invalidate any in-flight owner loads from the previous sort/filter so their
		// late responses can't overwrite the fresh ones.
		treeGen++
		const toReload = treeLazyMode ? [...openOwners] : []
		// Only the open owners are re-fetched below; a collapsed one keeps its rows as a
		// cache, which this reload invalidates. Left in place they would outlive the scope
		// they were loaded for: the owner stays grouped (so it keeps a node the new counts
		// say is empty) and, since it is still marked loaded, expanding it again shows the
		// previous scope's items instead of re-fetching. Drop them and let expand reload.
		if (treeLazyMode) {
			const open = new Set(toReload)
			treeOwnerItems = treeOwnerItems.filter((x) => open.has(ownerOf(effectivePath(x))))
			ownerLoad = Object.fromEntries(Object.entries(ownerLoad).filter(([o]) => open.has(o)))
		}
		await loadRunnables(true)
		// force: the prefixes are still marked loaded, so re-fetch their first page and
		// swap it in place (loadOwnerItems replaces a prefix's rows atomically — the old
		// rows stay visible until the new ones arrive, so nothing blanks mid-reorder).
		// Shallowest first, one depth at a time: a fresh load drops every row under its
		// prefix, so a parent landing after a nested subfolder would wipe the rows that
		// subfolder just re-fetched and leave it short until clicked again.
		// Awaited so a caller reconciling against the rendered rows sees the reloaded
		// tree rather than the pre-reload ones; each swap stays atomic either way.
		const byDepth = new Map<number, string[]>()
		for (const p of toReload) {
			const d = p.split('/').length
			byDepth.set(d, [...(byDepth.get(d) ?? []), p])
		}
		for (const d of [...byDepth.keys()].sort((a, b) => a - b)) {
			await Promise.all(
				(byDepth.get(d) ?? []).map((p) => loadOwnerItems(p, false, { force: true }))
			)
		}
	}

	// For row mutations (create/delete/move/archive), which also change how many
	// runnables an owner holds. A scope change (sort/archive/kind/…) doesn't go
	// through here: the counts resource keys on those itself.
	async function reloadItemsAndCounts(): Promise<void> {
		// A mutated row can be gone, or sit at a new path, afterwards: snapshot what
		// was on screen so the selection can drop what this reload removes instead of
		// keeping a dead path. `tick` lets the reloaded rows re-register first.
		const renderedBefore = homeSelection.renderedKeys
		void ownerCountsRes.refetch()
		await reloadItems()
		await tick()
		homeSelection.dropVanished(renderedBefore)
	}

	function filterItemsPathsBaseOnUserFilters(
		item: TableScript | TableFlow | TableApp | TableRawApp,
		filterUserFolders: boolean,
		filterUserFoldersType: 'only f/*' | 'u/username and f/*' | undefined
	) {
		if (!filterUserFoldersType || !filterUserFolders) return true
		const path = effectivePath(item)
		if (filterUserFoldersType === 'only f/*') return path.startsWith('f/')
		if (filterUserFoldersType === 'u/username and f/*')
			return path.startsWith('f/') || path.startsWith(`u/${$userStore?.username}/`)
		return true // should not happen
	}

	// The whole data layer below reads these two derived views of the searchbar filters, so
	// keep them the single source. Empty string reads as "no filter".
	let ownerFilter = $derived((filterValues.val.owner || undefined) as string | undefined)
	let labelFilter = $derived((filterValues.val.label || undefined) as string | undefined)
	// Chip-row setters. Clearing deletes the key rather than writing null, which the
	// searchbar would otherwise render as a `key: null` tag.
	function setOwnerFilter(o: string | undefined) {
		if (o == undefined) delete filterValues.val.owner
		else filterValues.val.owner = o
	}
	function setLabelFilter(l: string | undefined) {
		if (l == undefined) delete filterValues.val.label
		else filterValues.val.label = l
	}

	const cmp = new Intl.Collator('en').compare

	// The selected order maps to the endpoint's order_by/order_desc (see
	// sortToParams) so it is applied server-side and stays correct across the
	// whole workspace, not just loaded pages. Starred items are pinned on top of
	// the first page.
	type SortOrder = 'updated_desc' | 'updated_asc' | 'name_asc' | 'name_desc'
	const SORT_SETTING_NAME = 'homeSort'
	// `short` labels the trigger button next to the sort icon (the button is icon-only
	// only while searching, when sorting is disabled — see below).
	const sortOptions: { value: SortOrder; label: string; short: string }[] = [
		{ value: 'updated_desc', label: 'Recently updated', short: 'Recent' },
		{ value: 'updated_asc', label: 'Oldest updated', short: 'Oldest' },
		{ value: 'name_asc', label: 'Name (A-Z)', short: 'A-Z' },
		{ value: 'name_desc', label: 'Name (Z-A)', short: 'Z-A' }
	]
	let sortOrder = $state<SortOrder>(
		sortOptions.find((o) => o.value === getLocalSetting(SORT_SETTING_NAME))?.value ?? 'updated_desc'
	)
	$effect(() => {
		storeLocalSetting(SORT_SETTING_NAME, sortOrder === 'updated_desc' ? undefined : sortOrder)
	})
	let sortItems: MenuItem[] = $derived(
		sortOptions.map((o) => ({
			displayName: o.label,
			selected: o.value === sortOrder,
			action: () => (sortOrder = o.value)
		}))
	)
	// Preserve the endpoint's exact order rather than re-deriving it on the client:
	// each row carries its server fetch ordinal (`ord`), which already reflects the
	// chosen order, the (path, kind) tiebreaks, full-precision timestamps, the database
	// collation, and starred-first pinning. Sorting by it can never disagree with the
	// server, so a later page never jumps above shown rows on "load more". Depends on
	// `sortOrder` only so its identity changes when the order does (re-grouping the tree,
	// whose leaves also sort by ord); the actual reordering comes from the reload that
	// restamps ord.
	let compareItems = $derived.by(() => {
		sortOrder
		return (a: { ord?: number }, b: { ord?: number }): number => (a.ord ?? 0) - (b.ord ?? 0)
	})

	const opts: uFuzzy.Options = {
		sort: (info, haystack, needle) => {
			let {
				idx,
				chars,
				terms,
				interLft2,
				interLft1,
				//	interRgt2,
				//	interRgt1,
				start,
				intraIns,
				interIns
			} = info

			const sortResult = idx
				.map((v, i) => i)
				.sort(
					(ia, ib) =>
						// most contig chars matched
						chars[ib] - chars[ia] ||
						// least char intra-fuzz (most contiguous)
						intraIns[ia] - intraIns[ib] ||
						// most prefix bounds, boosted by full term matches
						terms[ib] +
							interLft2[ib] +
							0.5 * interLft1[ib] -
							(terms[ia] + interLft2[ia] + 0.5 * interLft1[ia]) ||
						// highest density of match (least span)
						//	span[ia] - span[ib] ||
						// highest density of match (least term inter-fuzz)
						interIns[ia] - interIns[ib] ||
						// earliest start of match
						start[ia] - start[ib] ||
						// alphabetic
						cmp(haystack[idx[ia]], haystack[idx[ib]]) +
							(preFilteredItems?.[idx[ib]]?.starred ? 100 : 0) -
							(preFilteredItems?.[idx[ia]]?.starred ? 100 : 0)
				)
			return sortResult
		}
	}

	function resetScroll() {
		const element = document.getElementsByTagName('svelte-virtual-list-viewport')
		const firstElement = element.item(0)
		if (firstElement) {
			firstElement.scrollTop = 0
		}
	}

	const TREE_VIEW_SETTING_NAME = 'treeView'
	let treeView = $state(getLocalSetting(TREE_VIEW_SETTING_NAME) == 'true')

	// Pipeline entries are rendered independently of the item list, so apply the
	// same gates the items get — otherwise a pipeline would still show under the
	// Flows/Apps tabs, in the archived view, under a label filter, or outside a
	// selected owner. Pipelines are script-based units always at `f/<folder>`, so
	// kind=script and the user-folder toggle always include them; kind=flow/app,
	// archived, a label filter (pipelines carry no labels), and a non-matching
	// owner exclude them.
	let visiblePipelineFolders = $derived.by(() => {
		if (archived) return new Set<string>()
		if (itemKind !== 'all' && itemKind !== 'script') return new Set<string>()
		if (labelFilter != undefined) return new Set<string>()
		if (ownerFilter == undefined) return pipelineFolders
		return new Set(
			[...pipelineFolders].filter(
				(f) => `f/${f}` === ownerFilter || `f/${f}`.startsWith(ownerFilter + '/')
			)
		)
	})
	let viewCodeDrawer: Drawer | undefined = $state()
	let viewCodeTitle: string | undefined = $state()
	let script: Script | undefined = $state()
	async function showCode(path: string, summary: string) {
		viewCodeTitle = summary || path
		await viewCodeDrawer?.openDrawer()
		// `getDraft: true` so draft-only scripts (no deployed row at this
		// path) still return their content via the per-user draft overlay
		// instead of 404'ing.
		script = await ScriptService.getScriptByPath({
			workspace: $workspaceStore!,
			path,
			getDraft: true
		})
	}

	let collapseAll = $state(true)
	// Human-readable list of the filters currently narrowing the list. Empty means no
	// filter is active, so an empty result then means the workspace itself has no items
	// (NoItemFound shows the welcome message); otherwise the filters are just too narrow
	// and NoItemFound lists them.
	let activeFilters = $derived.by(() => {
		const f: string[] = []
		if (filter !== '') f.push(`search “${filter}”`)
		if (itemKind !== 'all')
			f.push(itemKind === 'script' ? 'Scripts' : itemKind === 'flow' ? 'Flows' : 'Apps')
		if (ownerFilter != undefined) f.push(ownerFilter)
		if (labelFilter != undefined) f.push(`label “${labelFilter}”`)
		if (archived) f.push('archived only')
		if (filterUserFolders)
			f.push(
				filterUserFoldersType === 'only f/*'
					? 'only f/* folders'
					: `only f/* and u/${$userStore?.username}`
			)
		if (!includeWithoutMain) f.push('library scripts hidden')
		return f
	})
	// Pipeline folders qualify for a chip whenever a pipeline can render: the kind must admit
	// one and no label filter may be active, since pipelines carry no labels. Unlike
	// `visiblePipelineFolders` this ignores the selected owner — the chips are how you switch
	// owners, so they must not narrow to the current one.
	let chipPipelineFolders = $derived(
		(itemKind === 'all' || itemKind === 'script') && labelFilter == undefined
			? pipelineFolders
			: new Set<string>()
	)
	// Owner chips: only the owners actually holding something the user can see, your own
	// space first and the rest most-populated first. A chip for an empty owner filters to
	// nothing, and a workspace's full folder and member lists are mostly those, so an owner
	// counting 0 gets no chip at all — including your own space.
	let owners = $derived.by(() => {
		const self = $userStore?.username ? `u/${$userStore.username}` : undefined
		const loaded = filteredItems?.map((x) => ownerOf(effectivePath(x))) ?? []
		if (ownerCounts == undefined) {
			// Counts still in flight: the folder/user lists resolve first, so painting the full
			// list here would show the wall this drops and snap to the ranked set a tick later.
			if (!archived && ownerCountsRes.loading) return []
			// No counts (archived view, or the request failed): every owner, alphabetically.
			return Array.from(new Set([...allFolderOwners, ...allUserOwners, ...loaded])).sort()
		}
		const counted = new Map<string, number>(Object.entries(ownerCounts))
		// The pipeline is a row of its own and its member scripts are folded out of the
		// count, so it adds one where it renders — as in the tree node's own label.
		for (const f of chipPipelineFolders) counted.set(`f/${f}`, (counted.get(`f/${f}`) ?? 0) + 1)
		// No owner counts below what the loaded window already shows for it: the endpoint
		// leaves pipeline members out, and a listed item must keep its chip.
		const onScreen = new Map<string, number>()
		for (const o of loaded) onScreen.set(o, (onScreen.get(o) ?? 0) + 1)
		for (const [o, n] of onScreen) counted.set(o, Math.max(counted.get(o) ?? 0, n))
		return (
			[...counted.keys()]
				// The user-folder restriction drops other users' rows from the list, so their chips
				// would filter to nothing — the same rule `filterItemsPathsBaseOnUserFilters` applies.
				.filter(
					(o) =>
						!filterUserFolders ||
						!filterUserFoldersType ||
						o.startsWith('f/') ||
						(filterUserFoldersType === 'u/username and f/*' && o === self)
				)
				.sort((a, b) => {
					if (a === self) return -1
					if (b === self) return 1
					return (counted.get(b) ?? 0) - (counted.get(a) ?? 0) || cmp(a, b)
				})
		)
	})
	// Reload from the server whenever an input the endpoint resolves changes: order,
	// archived/library scope, kind, the selected owner/folder, or entering/leaving
	// search (see the reload effect below). Only the label filter and fuzzy ranking
	// stay client-side over the loaded pages, so they don't reload here.
	let searching = $derived(filter !== '')
	// Lazy owner tree is active only in the tree view when browsing all (no owner
	// selected, no search, no label filter): every folder AND user shows as a top-level
	// node and paginates on expand, its items coming from the separate `treeOwnerItems`
	// store. In the flat view, or a selected owner / active search / label filter, the
	// lazy store is unused (the flat list and reloadItems must not touch it), so this is
	// false and those views group the global loaded window instead.
	let treeLazyMode = $derived(
		treeView && !searching && ownerFilter == undefined && labelFilter == undefined
	)
	// How many runnables each owner (`f/<folder>` / `u/<user>`) holds for this user,
	// in one request. It labels every tree node up front — a lazy owner's own count is
	// unknown until it's expanded — and lets both the tree and the owner chips drop the
	// owners holding nothing instead of listing every workspace folder. Owners with none
	// are omitted from the response, so an absent key means empty. Fetched in every mode
	// (the chips are shown in all of them) except the archived view, which the endpoint
	// doesn't count.
	let ownerCountsRes = resource(
		[() => $workspaceStore, () => archived, () => itemKind, () => includeWithoutMain],
		async ([ws, showArchived, kind, withoutMain]) => {
			if (!ws || showArchived) return undefined
			try {
				const res = await ScriptService.countRunnablesByOwner({
					workspace: ws,
					kinds: kind !== 'all' ? kind : undefined,
					includeWithoutMain: withoutMain ? true : undefined,
					// Same scope as the listing, so a badge counts the rows behind it.
					includeDraftOnly: true
				})
				return res.counts
			} catch {
				// Best-effort: without counts the tree and the chips fall back to every owner.
				return undefined
			}
		}
	)
	let ownerCounts = $derived(ownerCountsRes.current)
	// The counts decide which owners the tree renders, so drawing it before they land
	// would show every workspace folder and then prune it away. Hold the skeleton
	// until the first response instead — it is fetched in parallel with the listing,
	// so it costs no extra wait in practice. Only the first load gates: `current`
	// survives a refetch, so an in-place scope change refreshes without flashing.
	let treeCountsPending = $derived(
		treeLazyMode && ownerCountsRes.current == undefined && ownerCountsRes.loading
	)

	// Owners the counts found the user has something in, split by kind. They cover
	// what the folder/username lists miss: an item shared individually out of a
	// folder or user space the user is otherwise not a member of.
	function countOwners(kind: 'f' | 'u'): string[] {
		return Object.keys(ownerCounts ?? {})
			.filter((k) => k.startsWith(`${kind}/`))
			.map((k) => k.slice(2))
	}
	let treeInjectFolders = $derived(
		treeLazyMode ? [...new Set([...(folderNamesRes.current ?? []), ...countOwners('f')])] : []
	)
	let treeInjectUsers = $derived.by(() => {
		// "Only f/*" hides every user namespace; "u/<you> and f/*" keeps just your own.
		if (!treeLazyMode) return []
		if (filterUserFolders && filterUserFoldersType === 'only f/*') return []
		const s = new Set<string>()
		// Always inject your own personal space (u/<you>): list_usernames can be empty
		// (you may not be a listed workspace member) yet u/<you> still holds runnables,
		// and it must not vanish under a name sort whose first page is all folders.
		if ($userStore?.username) s.add($userStore.username)
		// Other users only when no user-folder restriction is active.
		if (!filterUserFolders) {
			for (const u of usernamesRes.current ?? []) s.add(u)
			for (const u of countOwners('u')) s.add(u)
		}
		return [...s]
	})
	// The bottom "load more" only pages the *global* stream, which in lazy mode holds
	// folder rows the tree ignores (folders come from the store and are all injected
	// already) — so surfacing it there is confusing. Restrict it to scoped mode, where
	// it pages within the selected owner. In lazy mode the footer instead reveals more
	// root nodes purely client-side (nbDisplayed) when there are more than are shown,
	// and each folder paginates within itself; users past the window are reached via
	// their owner chip.
	let treeGlobalHasMore = $derived(ownerFilter != undefined && !searching ? hasMoreServer : false)
	$effect(() => {
		if ($userStore && $workspaceStore) {
			;[archived, includeWithoutMain, sortOrder, searching, ownerFilter, itemKind]
			// reloadItems (not a bare loadRunnables) so an in-place change — sort, archive,
			// library, kind — reloads the global stream AND re-fetches the currently-open
			// owners in the new order/filter, keeping folders expanded (file-explorer style)
			// rather than collapsing them. Mode changes (owner/search) flip treeLazyMode, so
			// reloadItems skips the re-fetch and the {#key} remount handles the reset.
			untrack(() => {
				reloadItems()
			})
		}
	})

	// Debounced server-side search augmentation. Instant filtering stays fully
	// client-side (SearchItems over the loaded pages) for reactivity; this fetches ONE
	// page of matches beyond the loaded browse window and, if the server has more,
	// exposes them through an explicit "load more results" control (searchCursor) rather
	// than auto-downloading the whole workspace for a broad term.
	let searchCursor = $state<string | undefined>(undefined)
	let searchLoadingMore = $state(false)
	$effect(() => {
		const term = filter
		const ws = $workspaceStore
		// Same view scope as the browse list, so a search can't surface archived /
		// library items the current view excludes, and stays within the selected folder.
		const showArchived = archived
		const withoutMain = includeWithoutMain
		const owner = ownerFilter
		const kind = itemKind
		// Any term/scope change restarts search paging.
		searchCursor = undefined
		if (term === '' || !ws || !$userStore) return
		const handle = setTimeout(async () => {
			let res: { items: RunnableItem[]; next_cursor?: string }
			try {
				res = await ScriptService.listRunnables({
					workspace: ws,
					search: term,
					showArchived: showArchived ? true : undefined,
					includeWithoutMain: withoutMain ? true : undefined,
					kinds: kind !== 'all' ? kind : undefined,
					pathStart: owner ? owner + '/' : undefined,
					includeDraftOnly: true,
					perPage: 1000
				})
			} catch {
				return
			}
			// Drop a stale response: workspace, term, or view scope moved on while it was
			// in flight (the debounce cancels superseded timers; this guards the request).
			if (
				untrack(() => $workspaceStore) !== ws ||
				untrack(() => filter) !== term ||
				untrack(() => archived) !== showArchived ||
				untrack(() => includeWithoutMain) !== withoutMain ||
				untrack(() => ownerFilter) !== owner ||
				untrack(() => itemKind) !== kind
			)
				return
			mergeRunnables(res.items ?? [])
			searchCursor = res.next_cursor
		}, 300)
		return () => clearTimeout(handle)
	})

	// Fetch the next page of search matches on demand — one page per click, so a broad
	// search stays complete without auto-loading the entire workspace into memory.
	async function loadMoreSearchResults(): Promise<void> {
		// Capture the full scope: a cursor only encodes the last row's sort keys, so two
		// different scopes can mint the same cursor — the response must be rejected unless
		// EVERY scope input still matches, not just workspace + cursor.
		const ws = $workspaceStore
		const term = filter
		const showArchived = archived
		const withoutMain = includeWithoutMain
		const owner = ownerFilter
		const kind = itemKind
		const cursor = searchCursor
		if (!cursor || !ws || term === '' || searchLoadingMore) return
		searchLoadingMore = true
		let res: { items: RunnableItem[]; next_cursor?: string }
		try {
			res = await ScriptService.listRunnables({
				workspace: ws,
				search: term,
				showArchived: showArchived ? true : undefined,
				includeWithoutMain: withoutMain ? true : undefined,
				kinds: kind !== 'all' ? kind : undefined,
				pathStart: owner ? owner + '/' : undefined,
				includeDraftOnly: true,
				perPage: 1000,
				cursor
			})
		} catch {
			searchLoadingMore = false
			return
		}
		searchLoadingMore = false
		// Discard unless the term AND every view scope are still the ones we fetched for.
		if (
			untrack(() => $workspaceStore) !== ws ||
			untrack(() => filter) !== term ||
			untrack(() => archived) !== showArchived ||
			untrack(() => includeWithoutMain) !== withoutMain ||
			untrack(() => ownerFilter) !== owner ||
			untrack(() => itemKind) !== kind ||
			untrack(() => searchCursor) !== cursor
		)
			return
		mergeRunnables(res.items ?? [])
		searchCursor = res.next_cursor
		// Reveal the freshly fetched matches instead of leaving them behind the display cap.
		nbDisplayed += 30
	}

	let combinedItems = $derived(
		flows == undefined || scripts == undefined || apps == undefined || raw_apps == undefined
			? undefined
			: [
					...flows.map((x) => ({
						...x,
						type: 'flow' as 'flow',
						time: new Date(x.edited_at).getTime()
					})),
					...scripts.map((x) => ({
						...x,
						type: 'script' as 'script',
						time: new Date(x.created_at).getTime()
					})),
					...apps.map((x) => ({
						...x,
						type: 'app' as 'app',
						time: new Date(x.edited_at).getTime()
					})),
					...raw_apps.map((x) => ({
						...x,
						type: 'raw_app' as 'raw_app',
						time: new Date(x.edited_at).getTime()
					}))
				].sort(compareItems)
	)
	function itemLabels(x: { labels?: string[]; inherited_labels?: string[] }): string[] {
		return [...(x.labels ?? []), ...(x.inherited_labels ?? [])]
	}
	// Labels ranked by how many loaded rows carry them (ties alphabetical). Unlike the owner
	// chips there is no workspace-wide count endpoint, so the order is window-local and can
	// shift as later pages load. A row carrying a label both directly and by inheritance
	// counts once.
	let allLabels = $derived.by(() => {
		const counts = new Map<string, number>()
		for (const x of combinedItems ?? [])
			for (const l of new Set(itemLabels(x))) counts.set(l, (counts.get(l) ?? 0) + 1)
		return [...counts.keys()].sort(
			(a, b) => (counts.get(b) ?? 0) - (counts.get(a) ?? 0) || cmp(a, b)
		)
	})
	let hasChips = $derived(
		owners.length > 0 ||
			allLabels.length > 0 ||
			ownerFilter != undefined ||
			labelFilter != undefined
	)
	// FilterSearchbar presets: the owner prefixes and labels the list actually holds, so
	// scoping to one is a click in the searchbar dropdown.
	// Owner sets the `owner` filter (server path-scope), label sets `label` (client filter).
	// The `:\ ` separator and escaped spaces match the canonical `key:\ value` form parseToText
	// emits, so the "already applied" check finds them after a reparse and won't re-offer a
	// duplicate.
	let searchPresets = $derived([
		...owners.map((o) => ({ name: o, value: `owner:\\ ${o.replace(/ /g, '\\ ')}` })),
		...allLabels.map((l) => ({ name: l, value: `label:\\ ${l.replace(/ /g, '\\ ')}` }))
	])
	let prevWorkspace: string | undefined = undefined
	// An owner/label from one workspace means nothing in another, so drop them when the
	// workspace actually changes. The initial resolution is left alone so URL-loaded filter
	// values survive the async store settling.
	$effect(() => {
		const ws = $workspaceStore
		if (ws && prevWorkspace !== undefined && ws !== prevWorkspace) {
			delete filterValues.val.owner
			delete filterValues.val.label
		}
		prevWorkspace = ws
	})
	// The kind and owner/folder filters are resolved server-side (kinds, path_start),
	// so only the user-folder toggle and label filters run client-side here.
	let preFilteredItems = $derived(
		combinedItems?.filter(
			(x) =>
				filterItemsPathsBaseOnUserFilters(x, filterUserFolders, filterUserFoldersType) &&
				(labelFilter == undefined || itemLabels(x).includes(labelFilter))
		)
	)
	let items = $derived(filter !== '' ? filteredItems : preFilteredItems)
	// Source the tree groups. In lazy mode every top-level owner (folder and user) is
	// injected as a node and its rows come from the on-demand `treeOwnerItems` store,
	// so the tree never depends on which owners happen to be in the loaded window.
	// Otherwise (scoped/search/label) the tree just groups the global `items`.
	let treeSource = $derived(
		treeLazyMode
			? treeOwnerItems.filter((x) =>
					filterItemsPathsBaseOnUserFilters(x, filterUserFolders, filterUserFoldersType)
				)
			: items
	)
	// Remount identity: only a *mode* change (workspace, selected owner, entering/leaving
	// search, label filter) restructures the tree, so only those key the {#key} remount.
	// A sort/archive/library/kind change keeps the same lazy tree and is applied in place
	// by reloadItems re-fetching the open owners — so expanded folders don't collapse when
	// you re-sort (searching is a boolean so per-keystroke query changes don't remount).
	let treeKey = $derived(
		`${treeView}|${$workspaceStore}|${ownerFilter}|${searching}|${labelFilter}`
	)
	// A mode change restructures the tree (it remounts on treeKey), so the lazy owner
	// store is stale — clear it here (the global reset no longer does). An in-place
	// sort/filter change leaves treeKey unchanged, so the store survives and its owners
	// are refreshed in place instead of blanking.
	$effect(() => {
		treeKey
		untrack(() => {
			treeGen++
			ownerLoad = {}
			treeOwnerItems = []
			openOwners = new Set()
		})
	})
	let displayedItems = $derived((items ?? []).slice(0, nbDisplayed))
	$effect(() => {
		items && resetScroll()
	})

	let selectedIndex: number = $state(-1)
	// More to show: either loaded items not yet sliced in, or the server has further
	// browse pages (not while searching — the browse cursor is paused then; further
	// search matches load on demand via searchCursor / "Load more results").
	let hasMore = $derived(
		items != undefined && (items.length > nbDisplayed || (hasMoreServer && !searching))
	)
	let loadMoreIndex = $derived(displayedItems.length)
	let loadMoreEl: HTMLButtonElement | undefined = $state()
	let pendingAutoSelect = $state(true)
	let firstWorkspaceRun = true
	$effect(() => {
		$workspaceStore
		pendingAutoSelect = true
		if (firstWorkspaceRun) {
			firstWorkspaceRun = false
			return
		}
		// On workspace switch, melt-ui restores focus to the workspace-picker trigger
		// button asynchronously after the menu closes. Without overriding it, pressing
		// an arrow key would re-open / re-highlight the workspace picker instead of
		// moving the items-list selection. Run several times to win the focus race.
		const focusSearch = () => {
			const el = document.getElementById('home-search-input') as HTMLInputElement | null
			el?.focus()
		}
		focusSearch()
		const raf1 = requestAnimationFrame(() => {
			focusSearch()
			requestAnimationFrame(focusSearch)
		})
		const timeoutId = setTimeout(focusSearch, 100)
		return () => {
			cancelAnimationFrame(raf1)
			clearTimeout(timeoutId)
		}
	})
	$effect(() => {
		filter
		itemKind
		ownerFilter
		labelFilter
		// Skip while pendingAutoSelect is true (initial load / workspace switch);
		// the auto-select effect below will set the index once items appear.
		if (!pendingAutoSelect) {
			selectedIndex = -1
		}
	})
	$effect(() => {
		if (pendingAutoSelect && displayedItems.length > 0) {
			selectedIndex = 0
			pendingAutoSelect = false
		}
	})
	$effect(() => {
		const max = hasMore ? displayedItems.length : displayedItems.length - 1
		if (selectedIndex > max) {
			selectedIndex = max
		}
	})
	$effect(() => {
		if (hasMore && selectedIndex === loadMoreIndex) {
			loadMoreEl?.scrollIntoView({ block: 'nearest' })
		}
	})
	// Capture-phase listener so we run before melt-ui's button keydown handlers
	// (e.g. ArrowDown on the dropdown trigger would otherwise open the menu).
	$effect(() => {
		window.addEventListener('keydown', handleGlobalKeydown, true)
		return () => window.removeEventListener('keydown', handleGlobalKeydown, true)
	})

	async function loadMore() {
		// Fetch the next server page once all loaded items are already sliced in.
		if (items && nbDisplayed >= items.length && hasMoreServer && !searching) {
			await loadRunnables(false)
		}
		nbDisplayed += 30
	}

	// Fetch the next server page directly (tree view and the empty-state control
	// use this — their "all shown" threshold is on a different scale than the flat
	// list's, so they must not route through loadMore's item-count check).
	async function fetchMoreServer() {
		if (hasMoreServer && !searching) {
			await loadRunnables(false)
			nbDisplayed += 30
		}
	}

	async function loadMoreAndPreselectFirstNew() {
		const previousNbDisplayed = nbDisplayed
		await loadMore()
		selectedIndex = previousNbDisplayed
	}

	// The searchbar is a contenteditable, not an <input>, so it can't be matched by SKIP_SELECTOR
	// and has no `.value`/`.selectionEnd`. It owns the arrows only while its suggestion dropdown is
	// open (free-text mode passes them through to the list); track that so nav stands down then.
	let searchbarDropdownOpen = $state(false)

	// Caret position inside the searchbar's contenteditable, via the Selection API — the equivalent
	// of an <input>'s selectionStart/End the list navigation used before the searchbar swap.
	function searchCaret(el: HTMLElement): { atStart: boolean; atEnd: boolean; empty: boolean } {
		const text = el.textContent ?? ''
		const sel = window.getSelection()
		if (!sel || sel.rangeCount === 0)
			return { atStart: true, atEnd: true, empty: text.length === 0 }
		const range = sel.getRangeAt(0)
		const pre = range.cloneRange()
		pre.selectNodeContents(el)
		pre.setEnd(range.endContainer, range.endOffset)
		const caret = pre.toString().length
		return { atStart: caret === 0, atEnd: caret >= text.length, empty: text.length === 0 }
	}

	// Elements that own the keyboard themselves (menus, dialogs, comboboxes): the
	// list's own shortcuts stand down while one of them has focus.
	const SKIP_SELECTOR =
		'[role="menu"], [role="menuitem"], [role="dialog"], [role="listbox"], [role="combobox"], [aria-expanded="true"], [data-menu], [data-chat-keyboard-scope]'

	// The marker sits on the row itself, not on its title link — selection mode
	// drops the link, and the action buttons must stay keyboard-reachable there too.
	function getSelectedRowActionButtons(): HTMLElement[] {
		const actions = document.querySelector<HTMLElement>(
			'[data-row-keyboard-selected="true"] [data-row-actions]'
		)
		return actions ? Array.from(actions.querySelectorAll<HTMLElement>('button, a[href]')) : []
	}

	function handleGlobalKeydown(e: KeyboardEvent) {
		// An open dialog owns the keyboard. Testing the focused element alone misses it
		// (a modal opened from a button leaves focus on that button), and this capture
		// listener runs before the dialog's, so Enter would tick a row into the batch
		// being confirmed. Dialogs are in the DOM only while open.
		if (document.querySelector('[role="dialog"]')) return

		const target = e.target as HTMLElement | null

		// Escape leaves selection mode from either view; everything below is flat-list
		// navigation. A menu that owns the key closes itself first.
		if (e.key === 'Escape' && homeSelection.active) {
			const active = document.activeElement as HTMLElement | null
			if (!target?.closest(SKIP_SELECTOR) && !active?.closest(SKIP_SELECTOR)) {
				e.preventDefault()
				homeSelection.exit()
				return
			}
		}
		if (treeView) return

		// When focus is inside a row's action buttons, handle arrow keys ourselves:
		//  - Left/Right cycle between buttons (Left from the first returns to search).
		//  - Up/Down move to the same-position button on the previous/next row.
		// All other keys pass through so Enter/Space activate the focused button normally.
		// This must run BEFORE the skipSelector check, since the dropdown ellipsis
		// trigger carries [data-menu] (which would otherwise filter the event out).
		// Up/Down also need stopImmediatePropagation so melt-ui's dropdown trigger
		// doesn't open the menu (its default ArrowDown behavior).
		const actionsContainer = target?.closest<HTMLElement>('[data-row-actions]')
		if (actionsContainer) {
			if (
				e.key !== 'ArrowRight' &&
				e.key !== 'ArrowLeft' &&
				e.key !== 'ArrowUp' &&
				e.key !== 'ArrowDown'
			)
				return
			const buttons = Array.from(actionsContainer.querySelectorAll<HTMLElement>('button, a[href]'))
			const currentIdx = buttons.indexOf(target as HTMLElement)
			if (currentIdx < 0) return
			if (e.key === 'ArrowRight') {
				if (currentIdx < buttons.length - 1) {
					e.preventDefault()
					buttons[currentIdx + 1].focus()
				}
			} else if (e.key === 'ArrowLeft') {
				e.preventDefault()
				if (currentIdx > 0) {
					buttons[currentIdx - 1].focus()
				} else {
					;(document.getElementById('home-search-input') as HTMLInputElement | null)?.focus()
				}
			} else {
				// ArrowUp / ArrowDown: move to same-position button on prev/next row.
				e.preventDefault()
				e.stopImmediatePropagation()
				if (selectedIndex < 0 || selectedIndex >= displayedItems.length) return
				const newIndex =
					e.key === 'ArrowDown'
						? Math.min(selectedIndex + 1, displayedItems.length - 1)
						: Math.max(selectedIndex - 1, 0)
				if (newIndex === selectedIndex) return
				selectedIndex = newIndex
				tick().then(() => {
					const newButtons = getSelectedRowActionButtons()
					if (newButtons.length === 0) return
					const targetIdx = Math.min(currentIdx, newButtons.length - 1)
					newButtons[targetIdx]?.focus()
				})
			}
			return
		}

		// Inside an open dropdown menu: ArrowUp on first item / ArrowDown on last item
		// closes the menu (so users can leave with arrows instead of needing Escape).
		// Other arrow keys fall through to melt-ui's default cycle.
		const menuItem = target?.closest<HTMLElement>('[role="menuitem"]')
		if (menuItem) {
			if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
				const menu = menuItem.closest<HTMLElement>('[role="menu"]')
				// menus marked data-arrow-loop keep melt's cyclic wrap instead of exiting
				if (menu && !menu.hasAttribute('data-arrow-loop')) {
					const items = Array.from(menu.querySelectorAll<HTMLElement>('[role="menuitem"]'))
					const idx = items.indexOf(menuItem)
					const isFirst = idx === 0
					const isLast = idx === items.length - 1
					if ((e.key === 'ArrowUp' && isFirst) || (e.key === 'ArrowDown' && isLast)) {
						e.preventDefault()
						e.stopImmediatePropagation()
						menuItem.dispatchEvent(
							new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true })
						)
					}
				}
			}
			return
		}

		const skipSelector = SKIP_SELECTOR
		if (target) {
			const tag = target.tagName
			const isEditable =
				tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || target.isContentEditable
			const isOurSearch = target.id === 'home-search-input'
			if (isEditable && !isOurSearch) return
			// While the searchbar's suggestion dropdown is open it owns the arrows/Enter itself.
			if (isOurSearch && searchbarDropdownOpen) return
			if (target.closest(skipSelector)) return
		}
		const active = document.activeElement as HTMLElement | null
		if (active?.closest(skipSelector)) return

		// ArrowRight from search input / body → focus first action button of selected row.
		// Guard: if cursor is in the middle of typed search text, let the cursor move.
		if (e.key === 'ArrowRight') {
			if (target?.id === 'home-search-input') {
				const c = searchCaret(target)
				if (!c.empty && !c.atEnd) return
			}
			if (selectedIndex < 0 || selectedIndex >= displayedItems.length) return
			const buttons = getSelectedRowActionButtons()
			if (buttons.length > 0) {
				e.preventDefault()
				buttons[0].focus()
			}
			return
		}
		// ArrowLeft from search input with cursor at start: no-op (let default handle).
		if (e.key === 'ArrowLeft') {
			if (target?.id === 'home-search-input') {
				const c = searchCaret(target)
				if (!c.empty && !c.atStart) return
			}
			return
		}

		if (e.key === 'ArrowDown') {
			if (displayedItems.length === 0) return
			e.preventDefault()
			if (selectedIndex === -1) {
				selectedIndex = 0
			} else if (selectedIndex === loadMoreIndex && hasMore) {
				selectedIndex = 0
			} else if (selectedIndex === displayedItems.length - 1) {
				selectedIndex = hasMore ? loadMoreIndex : 0
			} else {
				selectedIndex = selectedIndex + 1
			}
		} else if (e.key === 'ArrowUp') {
			if (displayedItems.length === 0) return
			e.preventDefault()
			if (selectedIndex === -1) {
				selectedIndex = displayedItems.length - 1
			} else if (selectedIndex === loadMoreIndex && hasMore) {
				selectedIndex = displayedItems.length - 1
			} else if (selectedIndex === 0) {
				selectedIndex = hasMore ? loadMoreIndex : displayedItems.length - 1
			} else {
				selectedIndex = selectedIndex - 1
			}
		} else if (e.key === 'Enter') {
			// Enter belongs to whatever control has focus — the action bar's buttons,
			// a row's own actions, a link. Claiming it there would both suppress that
			// control and act on the highlighted row instead.
			if (target?.closest('button, a[href], [role="button"]')) return
			// In selection mode the rows carry no link, so Enter ticks the highlighted
			// row instead of opening it. Never for a legacy raw-app row, which carries
			// no selection control — that falls through to opening it below.
			if (
				homeSelection.active &&
				selectedIndex >= 0 &&
				selectedIndex < displayedItems.length &&
				displayedItems[selectedIndex].type !== 'raw_app'
			) {
				e.preventDefault()
				homeSelection.toggle(
					toBulkItem(displayedItems[selectedIndex], $userStore, $workspaceStore),
					e.shiftKey
				)
			} else if (selectedIndex === loadMoreIndex && hasMore) {
				e.preventDefault()
				loadMoreAndPreselectFirstNew()
			} else if (selectedIndex >= 0 && selectedIndex < displayedItems.length) {
				// Direct child only: that is the title link. Selection mode drops it, and
				// a descendant match would find the row's Edit link and open the editor.
				const anchor = document.querySelector<HTMLAnchorElement>(
					'[data-row-keyboard-selected="true"] > a[href]'
				)
				if (anchor) {
					e.preventDefault()
					anchor.click()
				}
			}
		} else if (e.key === 'Escape') {
			if (selectedIndex !== -1) {
				e.preventDefault()
				selectedIndex = -1
			}
		}
	}
	$effect(() => {
		storeLocalSetting(TREE_VIEW_SETTING_NAME, treeView ? 'true' : undefined)
	})
	// Multi-selection + bulk actions. Published through context so the tree's
	// nested levels don't have to carry it; `Item` is the only reader.
	const homeSelection = new HomeSelection()
	setHomeSelection(homeSelection)
	$effect(() => {
		homeSelection.available = showEditButtons && !!$userStore && !$userStore.operator
	})
	// A selected path means nothing in another workspace. Narrowing the view
	// within one (kind, owner, label, search) keeps the selection instead, so
	// items can be gathered across several filters; every action lists the paths
	// it will touch, so nothing acts invisibly.
	$effect(() => {
		$workspaceStore
		untrack(() => homeSelection.exit())
	})
	// Only folders/user spaces the user owns: a move into any other lands as a
	// per-item permission error the user could have been spared.
	let moveTargets = $derived(
		[...allFolderOwners, ...($userStore?.username ? [`u/${$userStore.username}`] : [])].filter(
			(o) => isOwner(`${o}/x`, $userStore, $workspaceStore)
		)
	)
</script>

<SearchItems
	{filter}
	items={preFilteredItems}
	bind:filteredItems
	f={(x) => {
		// A draft-only row is named by the path typed in the editor — its stored path is a
		// generated `draft_<uuid>` nobody types into the search box.
		const p = x.draft_only && x.draft_path ? x.draft_path : x.path
		return x.summary ? x.summary + ' (' + p + ')' : p
	}}
	{opts}
/>

<Drawer
	bind:this={viewCodeDrawer}
	on:close={() => {
		setTimeout(() => {
			viewCodeTitle = undefined
			script = undefined
		}, 300)
	}}
>
	<DrawerContent title={viewCodeTitle} on:close={viewCodeDrawer.closeDrawer}>
		{#if script}
			<HighlightCode language={script?.language} code={script?.content} />
		{:else}
			<Skeleton layout={[[40]]} />
		{/if}
	</DrawerContent>
</Drawer>

<CenteredPage wrapperClasses="w-full" handleOverflow={false}>
	<div
		class="flex flex-wrap gap-2 items-center justify-between w-full"
		use:triggerableByAI={{
			id: 'home-items-list',
			description: 'Lists of scripts, flows, and apps'
		}}
	>
		{#if !contentActive}
			<div class="flex justify-start">
				<ToggleButtonGroup
					selected={itemKind}
					onSelected={(v) => {
						// itemKind is derived from the shared filter object (which URL-syncs itself);
						// `all` clears the kind filter (delete, not null, so it doesn't linger as a
						// `kind: null` chip).
						if (v === 'all') {
							delete filterValues.val.kind
						} else {
							filterValues.val.kind = v
							subtab = v
						}
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="all" label="All" size="md" {item} />
						<ToggleButton value="script" icon={Code2} label="Scripts" size="md" {item} />
						{#if HOME_SEARCH_SHOW_FLOW}
							<ToggleButton
								value="flow"
								label="Flows"
								icon={FlowIcon}
								selectedColor="#14b8a6"
								size="md"
								{item}
							/>
						{/if}
						<ToggleButton
							value="app"
							label="Apps"
							icon={LayoutDashboard}
							selectedColor="#fb923c"
							size="md"
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>
			</div>
		{/if}

		{#if !loading && !contentActive}
			<!-- List controls, between the kind toggle and the searchbar: select mode, tree
			     view, expand/collapse (tree only), sort. -->
			<div class="flex items-center gap-2">
				{#if homeSelection.available && !homeSelection.active}
					<Button
						startIcon={{ icon: CheckSquare }}
						iconOnly
						unifiedSize="xs"
						variant="default"
						title="Select items — move, archive, delete or discard several at once"
						on:click={() => homeSelection.enter()}
					/>
				{/if}
				<Toggle size="xs" bind:checked={treeView} options={{ right: 'Tree view' }} />
				{#if treeView}
					<Button
						unifiedSize="sm"
						variant="subtle"
						on:click={() => (collapseAll = !collapseAll)}
						startIcon={{ icon: collapseAll ? ChevronsUpDown : ChevronsDownUp }}
					>
						{#if collapseAll}
							Expand all
						{:else}
							Collapse all
						{/if}
					</Button>
				{/if}
				<DropdownV2
					items={sortItems}
					disabled={filter !== ''}
					placement="bottom-end"
					fixedHeight={false}
				>
					{#snippet buttonReplacement()}
						{@const active = sortOptions.find((o) => o.value === sortOrder)}
						{@const short = filter !== '' ? '' : (active?.short ?? '')}
						<Button
							nonCaptureEvent
							disabled={filter !== ''}
							iconOnly={short === ''}
							unifiedSize="xs"
							variant="default"
							startIcon={{ icon: ArrowDownUp }}
							title={filter !== ''
								? 'Sorting is disabled while searching (results are ranked by relevance)'
								: `Sort: ${active?.label ?? ''}`}
						>
							{#if short !== ''}{short}{/if}
						</Button>
					{/snippet}
				</DropdownV2>
			</div>
		{/if}

		<div class="flex grow items-center justify-end gap-2 min-w-0">
			<div class="relative text-primary w-full min-w-[200px] max-w-[26rem]">
				<FilterSearchbar
					schema={searchbarSchema}
					bind:value={filterValues.val}
					placeholder={HOME_SEARCH_PLACEHOLDER}
					presets={contentActive ? [] : searchPresets}
					autofocus
					hideDropdownOnFreeText
					inputId="home-search-input"
					onDropdownVisibleChange={(v) => (searchbarDropdownOpen = v)}
				/>
			</div>
			<!-- Same gate the old create actions used: hidden from operators and in workspaces
			     whose direct-deploy protection cleared showEditButtons (NoDirectDeployAlert), since
			     the menu itself does no permission check. -->
			{#if !$userStore?.operator && showEditButtons}
				<CreateActionsMenu />
			{/if}
		</div>
	</div>
	{#if !contentActive && hasChips}
		<!-- Owner and label chips on one line. Each function binding routes the chip's
		     selection into the searchbar key of the same name, and `queryName` points
		     ListFilters' own mount-time URL read at the param the filter instance syncs, so
		     the two writers agree. No `syncQuery`: the filter instance owns the URL. -->
		<div class="gap-2 w-full flex flex-wrap mt-3">
			<ListFilters
				inline
				bind:selectedFilter={() => ownerFilter, setOwnerFilter}
				filters={owners}
				queryName="owner"
				maxDisplayed={10}
			/>
			<ListFilters
				inline
				bind:selectedFilter={() => labelFilter, setLabelFilter}
				filters={allLabels}
				queryName="label"
				maxDisplayed={10}
				color="blue"
				icon={Tag}
			/>
		</div>
	{/if}
	{#if filteredItems?.length == 0}
		<div class="mt-10"></div>
	{/if}
	<div class="mt-3">
		{#if filterValues.val.content}
			<!-- Content filter: swap the normal list/tree for the content-match view (the same one
			     used by the Ctrl-K "Content" modal). It loads the workspace's scripts/flows/apps/
			     resources and matches their contents client-side — usable on any instance. Keyed by
			     workspace so a switch remounts a fresh instance and late in-flight responses from
			     the previous workspace can't land in it. -->
			<!-- -mx-2 cancels ContentSearchInner's own px-2 so its rows line up flush with the
			     runnable list instead of sitting slightly inset. -->
			<div class="-mx-2">
				{#key $workspaceStore}
					<ContentSearchInner bind:this={contentSearchEl} search={filterValues.val.content} />
				{/key}
			</div>
		{:else if filteredItems == undefined || treeCountsPending}
			<div class="mt-4"></div>
			<Skeleton layout={[[2], 1]} />
			{#each new Array(6) as _}
				<Skeleton layout={[[4], 0.5]} />
			{/each}
		{:else if filteredItems.length === 0 && (filter !== '' || visiblePipelineFolders.size === 0)}
			<!-- Pipelines aren't part of the text filter, so only fall through to show
			     them (list rows / injected tree folders) when not actively searching;
			     a no-match search still reads as empty. -->
			<NoItemFound {activeFilters} />
			{#if hasMoreServer && !searching}
				<!-- The active filter matched nothing on the loaded pages, but the server
				     has more: keep paging reachable so matches on later pages aren't lost. -->
				<div class="text-center text-xs text-secondary mt-2">
					<button class="text-primary hover:text-emphasis underline" onclick={fetchMoreServer}
						>Load more to search further</button
					>
				</div>
			{/if}
		{:else if treeView}
			<!-- Remount the tree on a MODE change only (treeKey = view/owner/search/label):
			     expanded folders (their `opened` state is local to each TreeView) collapse
			     and re-load fresh for the new mode. An in-place order/archive/library/kind
			     change keeps treeKey stable, so folders stay open and refresh via
			     reloadItems instead (see loadOwnerItems). -->
			{#key treeKey}
				<TreeViewRoot
					items={treeSource}
					{collapseAll}
					sortCompare={compareItems}
					groupDesc={sortOrder === 'name_desc'}
					hasMoreServer={treeGlobalHasMore}
					onLoadMore={fetchMoreServer}
					pipelineFolders={visiblePipelineFolders}
					allFolders={treeInjectFolders}
					allUsers={treeInjectUsers}
					ownerCounts={!searching && labelFilter == undefined ? ownerCounts : undefined}
					selfUsername={$userStore?.username}
					ownerLoad={treeLazyMode ? ownerLoad : undefined}
					onExpandOwner={treeLazyMode ? loadOwnerItems : undefined}
					onCollapseOwner={treeLazyMode ? collapseOwner : undefined}
					isSearching={filter !== ''}
					on:scriptChanged={reloadItemsAndCounts}
					on:flowChanged={reloadItemsAndCounts}
					on:appChanged={reloadItemsAndCounts}
					on:rawAppChanged={reloadItemsAndCounts}
					on:reload={reloadItemsAndCounts}
					{showCode}
					showEditButton={showEditButtons}
				/>
			{/key}
		{:else}
			<div class="border rounded-md bg-surface-tertiary">
				{#if filter === ''}
					{#each [...visiblePipelineFolders].sort() as folder (folder)}
						<a
							href="{base}/pipeline/{encodeURIComponent(folder)}"
							class="w-full inline-flex items-center gap-4 px-4 py-3 border-b last:border-b-0 hover:bg-surface-hover transition-colors text-sm first-of-type:rounded-t-md"
						>
							<NetworkIcon size={16} class="text-emerald-600 dark:text-emerald-400" />
							<span class="text-xs font-medium text-emphasis truncate">Pipeline · f/{folder}</span>
						</a>
					{/each}
				{/if}
				{#each displayedItems as item, i (item.type + '/' + item.path + (item.hash ? '/' + item.hash : ''))}
					<Item
						{item}
						on:scriptChanged={reloadItemsAndCounts}
						on:flowChanged={reloadItemsAndCounts}
						on:appChanged={reloadItemsAndCounts}
						on:rawAppChanged={reloadItemsAndCounts}
						on:reload={reloadItemsAndCounts}
						{showCode}
						showEditButton={showEditButtons}
						keyboardSelected={selectedIndex === i}
					/>
				{/each}
			</div>
			{#if items && hasMore}
				<span class="text-xs font-normal text-secondary"
					>{Math.min(nbDisplayed, items.length)} items{hasMoreServer && !searching
						? ''
						: ` out of ${items.length}`}
					<button
						bind:this={loadMoreEl}
						class="ml-4 text-xs font-normal text-primary hover:text-emphasis rounded px-1 {selectedIndex ===
						loadMoreIndex
							? 'bg-gray-200 dark:bg-gray-700 underline'
							: ''}"
						onclick={loadMore}>load 30 more</button
					></span
				>
			{/if}
		{/if}
		{#if searching && searchCursor}
			<!-- The server has further search matches beyond the page already merged in;
			     fetch them one page at a time on demand rather than auto-downloading a
			     broad query's entire result set. -->
			<div class="text-center text-xs text-secondary mt-2">
				{#if searchLoadingMore}
					Loading more results…
				{:else}
					<button class="text-primary hover:text-emphasis underline" onclick={loadMoreSearchResults}
						>Load more results</button
					>
				{/if}
			</div>
		{/if}
		{#if homeSelection.active}
			<!-- The bar floats over the page bottom; without this the last rows sit
			     under it with no way to scroll them clear. -->
			<div class="h-20"></div>
		{/if}
	</div>
</CenteredPage>

{#if homeSelection.active && $workspaceStore}
	<BulkActionsBar
		selection={homeSelection}
		workspace={$workspaceStore}
		isAdmin={!!($userStore?.is_admin || $userStore?.is_super_admin)}
		{moveTargets}
		onDone={reloadItemsAndCounts}
	/>
{/if}
