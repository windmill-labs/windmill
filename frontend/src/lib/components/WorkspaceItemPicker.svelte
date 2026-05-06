<script lang="ts" module>
	import type { Flow, ListableApp, Script } from '$lib/gen'

	export type WorkspaceItemKind = 'flow' | 'script' | 'app'

	/** Shape emitted by the picker's `onPick` callback. The `raw_app` flag is only
	 * meaningful when `kind === 'app'` and discriminates low-code (`false`) from
	 * raw apps (`true`); see `editPathFor` for routing. */
	export type WorkspaceItem = {
		path: string
		summary: string
		kind: WorkspaceItemKind
		raw_app?: boolean
	}

	type Kind = WorkspaceItemKind
	type Item = WorkspaceItem

	/** Resolve the editor URL for an item picked from the workspace picker.
	 * Apps split into low-code (`/apps/edit`) and raw (`/apps_raw/edit`). */
	export function editPathFor(item: WorkspaceItem): string {
		if (item.kind === 'flow') return `/flows/edit/${item.path}`
		if (item.kind === 'script') return `/scripts/edit/${item.path}`
		return item.raw_app ? `/apps_raw/edit/${item.path}` : `/apps/edit/${item.path}`
	}

	type WorkspaceCache = {
		flow?: Item[]
		script?: Item[]
		app?: Item[]
	}

	const cache = new Map<string, WorkspaceCache>()
	const inflight = new Map<string, Promise<Item[]>>()

	function cacheKey(workspace: string, kind: Kind) {
		return `${workspace}:${kind}`
	}

	async function loadKind(workspace: string, kind: Kind): Promise<Item[]> {
		const existing = cache.get(workspace)?.[kind]
		if (existing) return existing
		const key = cacheKey(workspace, kind)
		const flying = inflight.get(key)
		if (flying) return flying

		const promise = (async () => {
			const { ScriptService, FlowService, AppService } = await import('$lib/gen')
			let items: Item[]
			if (kind === 'flow') {
				const flows = await FlowService.listFlows({
					workspace,
					includeDraftOnly: true,
					withoutDescription: true
				})
				items = flows.map((f: Flow) => ({
					path: f.path,
					summary: f.summary ?? '',
					kind: 'flow' as const
				}))
			} else if (kind === 'script') {
				const scripts = await ScriptService.listScripts({
					workspace,
					includeDraftOnly: true,
					withoutDescription: true
				})
				items = scripts.map((s: Script) => ({
					path: s.path,
					summary: s.summary ?? '',
					kind: 'script' as const
				}))
			} else {
				const apps = await AppService.listApps({
					workspace,
					includeDraftOnly: true
				})
				items = apps.map((a: ListableApp) => ({
					path: a.path,
					summary: a.summary ?? '',
					kind: 'app' as const,
					raw_app: a.raw_app ?? false
				}))
			}
			const bucket = cache.get(workspace) ?? {}
			bucket[kind] = items
			cache.set(workspace, bucket)
			return items
		})()
		inflight.set(key, promise)
		try {
			return await promise
		} finally {
			inflight.delete(key)
		}
	}
</script>

<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { ChevronRight, Folder, Loader2, User } from 'lucide-svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import type uFuzzy from '@leeoniya/ufuzzy'

	type Kind_ = 'flow' | 'script' | 'app'

	interface Props {
		onPick: (item: WorkspaceItem) => void
		kinds?: Kind_[]
		/** Composite-keyed nodes to open on mount. e.g. `['kind:flow', 'scope:flow:f/demo']`. */
		initialOpen?: string[]
		/** Composite key to highlight on mount. Preserved even if not yet visible (e.g. while
		 * the kind is still loading). */
		initialHighlight?: string
	}

	let {
		onPick,
		kinds = ['flow', 'script', 'app'],
		initialOpen = [],
		initialHighlight
	}: Props = $props()

	let searchInput: TextInput | undefined = $state()
	let pickerRoot: HTMLElement | undefined = $state()

	/** Focus + select the search input. Callable from outside (e.g. via a popover's openFocus). */
	export function focus() {
		searchInput?.focus()
		searchInput?.select()
	}

	// Fallback for switching between sibling breadcrumb popovers (closeOnOtherPopoverOpen):
	// melt-ui's `openFocus` runs once during the close→open transition and the new
	// picker may not be mounted yet when it tries to query the search input. Retry
	// after the transition has settled.
	$effect(() => {
		setTimeout(() => focus(), 50)
	})

	const KIND_LABEL: Record<Kind_, string> = {
		flow: 'Flows',
		script: 'Scripts',
		app: 'Apps'
	}

	const kindKey = (k: Kind_) => `kind:${k}`
	const scopeKey = (k: Kind_, scope: string) => `scope:${k}:${scope}`
	const leafKey = (it: Item) => `leaf:${it.kind}:${it.path}`

	let filter = $state('')
	let openItems = $state<Set<string>>(new Set(initialOpen))

	// Seed from the module-level cache so kinds already fetched in this session
	// render their scopes/leaves on the first frame (no async microtask gap, no
	// flash of "kind label only" while opening).
	let loaded = $state<Partial<Record<Kind_, Item[]>>>(
		(() => {
			const ws = $workspaceStore && cache.get($workspaceStore)
			if (!ws) return {}
			const out: Partial<Record<Kind_, Item[]>> = {}
			for (const k of kinds) if (ws[k]) out[k] = ws[k]
			return out
		})()
	)
	let loadingKind = $state<Partial<Record<Kind_, boolean>>>({})

	async function ensureLoaded(kind: Kind_) {
		if (!$workspaceStore) return
		if (loaded[kind]) return
		loadingKind[kind] = true
		try {
			const items = await loadKind($workspaceStore, kind)
			loaded[kind] = items
		} finally {
			loadingKind[kind] = false
		}
	}

	// Lazy-load each kind on first expansion
	$effect(() => {
		for (const id of openItems) {
			if (id.startsWith('kind:')) {
				const k = id.slice(5) as Kind_
				if (kinds.includes(k)) ensureLoaded(k)
			}
		}
	})

	// While searching, ensure every kind in scope is loaded so the search is comprehensive.
	$effect(() => {
		if (filter.trim() !== '') {
			for (const k of kinds) {
				ensureLoaded(k)
			}
		}
	})

	const scopeOf = (path: string) => path.split('/').slice(0, 2).join('/')

	type DisplayItem = Item & { marked?: string }
	type SearchInput = Item & { _key: string }

	let allItems = $derived<SearchInput[]>(
		kinds.flatMap((k) => (loaded[k] ?? []).map((it) => ({ ...it, _key: `${k}:${it.path}` })))
	)

	let searchedItems: DisplayItem[] | undefined = $state(undefined)

	const searchOpts: uFuzzy.Options = {}

	let groupedByScope = $derived.by(() => {
		const out: Record<Kind_, Map<string, Item[]>> = {
			flow: new Map(),
			script: new Map(),
			app: new Map()
		}
		for (const k of kinds) {
			const list = loaded[k]
			if (!list) continue
			const map = out[k]
			for (const it of list) {
				const scope = scopeOf(it.path)
				const arr = map.get(scope) ?? []
				arr.push(it)
				map.set(scope, arr)
			}
			for (const arr of map.values()) {
				arr.sort((a, b) => a.path.localeCompare(b.path))
			}
		}
		return out
	})

	let scopesByKind = $derived.by(() => {
		const out: Record<Kind_, string[]> = { flow: [], script: [], app: [] }
		for (const k of kinds) {
			out[k] = Array.from(groupedByScope[k].keys()).sort((a, b) => {
				const af = a.startsWith('f/') ? 0 : 1
				const bf = b.startsWith('f/') ? 0 : 1
				if (af !== bf) return af - bf
				return a.localeCompare(b)
			})
		}
		return out
	})

	let isSearching = $derived(filter.trim() !== '')

	let searchResultsByKind = $derived.by(() => {
		const out: Record<Kind_, DisplayItem[]> = { flow: [], script: [], app: [] }
		if (!searchedItems) return out
		for (const it of searchedItems) {
			out[it.kind].push(it)
		}
		return out
	})

	function pick(it: Item) {
		onPick({ path: it.path, summary: it.summary, kind: it.kind, raw_app: it.raw_app })
	}

	type NavNode =
		| { type: 'kind'; key: string; kind: Kind_ }
		| { type: 'scope'; key: string; kind: Kind_; scope: string }
		| { type: 'leaf'; key: string; item: Item }

	let navNodes: NavNode[] = $derived.by(() => {
		if (isSearching) {
			return kinds.flatMap((k) =>
				searchResultsByKind[k].map((it): NavNode => ({ type: 'leaf', key: leafKey(it), item: it }))
			)
		}
		const nodes: NavNode[] = []
		for (const k of kinds) {
			const kKey = kindKey(k)
			nodes.push({ type: 'kind', key: kKey, kind: k })
			if (!openItems.has(kKey)) continue
			const items = loaded[k]
			if (!items) continue
			for (const scope of scopesByKind[k]) {
				const sKey = scopeKey(k, scope)
				nodes.push({ type: 'scope', key: sKey, kind: k, scope })
				if (!openItems.has(sKey)) continue
				for (const it of groupedByScope[k].get(scope) ?? []) {
					nodes.push({ type: 'leaf', key: leafKey(it), item: it })
				}
			}
		}
		return nodes
	})

	let highlightedKey = $state<string | undefined>(initialHighlight)

	// Default to the first node when nothing is highlighted yet. In search mode
	// we also reset to the first node when the current highlight is no longer
	// visible (the search result set changed). In tree mode we preserve a non-
	// visible highlight — it might become visible after a lazy load completes.
	$effect(() => {
		if (navNodes.length === 0) return
		if (isSearching) {
			if (!highlightedKey || !navNodes.some((n) => n.key === highlightedKey)) {
				highlightedKey = navNodes[0].key
			}
		} else if (!highlightedKey) {
			highlightedKey = navNodes[0].key
		}
	})

	// Once the caller-supplied highlight becomes visible, scroll it into view.
	$effect(() => {
		if (highlightedKey && navNodes.some((n) => n.key === highlightedKey)) {
			queueMicrotask(scrollHighlightIntoView)
		}
	})

	function scrollHighlightIntoView() {
		if (!pickerRoot || !highlightedKey) return
		const el = pickerRoot.querySelector<HTMLElement>(
			`[data-nav-key="${CSS.escape(highlightedKey)}"]`
		)
		el?.scrollIntoView({ block: 'nearest' })
	}

	function moveHighlight(delta: 1 | -1) {
		if (navNodes.length === 0) return
		const cur = navNodes.findIndex((n) => n.key === highlightedKey)
		const next = cur < 0 ? 0 : (cur + delta + navNodes.length) % navNodes.length
		highlightedKey = navNodes[next].key
		queueMicrotask(scrollHighlightIntoView)
	}

	function toggleOpen(key: string) {
		const next = new Set(openItems)
		if (next.has(key)) next.delete(key)
		else next.add(key)
		openItems = next
	}

	function activate(key: string | undefined) {
		if (!key) return
		const node = navNodes.find((n) => n.key === key)
		if (!node) return
		if (node.type === 'leaf') {
			pick(node.item)
		} else {
			toggleOpen(node.key)
		}
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault()
			e.stopPropagation()
			moveHighlight(1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			e.stopPropagation()
			moveHighlight(-1)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			e.stopPropagation()
			activate(highlightedKey)
		}
	}
</script>

<SearchItems
	{filter}
	items={isSearching ? allItems : []}
	bind:filteredItems={searchedItems}
	f={(x: SearchInput) => (x.summary ? `${x.summary} (${x.path})` : x.path)}
	opts={searchOpts}
/>

{#snippet leafRow(it: Item, secondary: string, paddingClass: string)}
	{@const key = leafKey(it)}
	{@const isHl = key === highlightedKey}
	<button
		type="button"
		data-nav-key={key}
		class="w-full text-left flex items-center gap-2 transition-colors {paddingClass} {isHl
			? 'bg-surface-hover'
			: ''}"
		onclick={() => pick(it)}
		onmouseenter={() => (highlightedKey = key)}
	>
		<RowIcon kind={it.kind} size={12} />
		<div class="min-w-0 flex-1">
			{#if it.summary}
				<div class="text-xs text-primary truncate">{it.summary}</div>
				<div class="text-2xs text-secondary font-normal font-mono truncate">{secondary}</div>
			{:else}
				<div class="text-xs text-primary font-mono truncate">{secondary}</div>
			{/if}
		</div>
	</button>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={pickerRoot}
	class="flex flex-col w-[420px] max-h-[60vh]"
	onkeydown={handleSearchKeydown}
>
	<div class="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
		<TextInput
			bind:this={searchInput}
			bind:value={filter}
			size="sm"
			inputProps={{
				placeholder: 'Search by name or summary...',
				'data-workspace-picker-search': ''
			}}
		/>
	</div>

	<div class="flex-1 overflow-y-auto">
		{#if isSearching}
			{@const total = (searchedItems ?? []).length}
			{#if !searchedItems}
				<div class="px-3 py-2 text-xs text-tertiary flex items-center gap-2">
					<Loader2 size={14} class="animate-spin" /> Searching…
				</div>
			{:else if total === 0}
				<div class="px-3 py-2 text-xs text-tertiary">No matches</div>
			{:else}
				{#each kinds as k (k)}
					{@const results = searchResultsByKind[k]}
					{#if results.length > 0}
						<div class="px-3 pt-3 pb-1 text-2xs uppercase tracking-wide text-tertiary font-medium">
							{KIND_LABEL[k]}
						</div>
						<ul class="pb-1">
							{#each results as it (leafKey(it))}
								<li>{@render leafRow(it, it.path, 'px-3 py-1.5')}</li>
							{/each}
						</ul>
					{/if}
				{/each}
			{/if}
		{:else}
			<div class="flex flex-col py-1">
				{#each kinds as k (k)}
					{@const kKey = kindKey(k)}
					{@const kindOpen = openItems.has(kKey)}
					{@const kindHl = kKey === highlightedKey}
					<button
						type="button"
						data-nav-key={kKey}
						aria-expanded={kindOpen}
						class="flex items-center gap-1.5 w-full text-left px-3 py-1.5 text-xs font-medium font-mono text-emphasis transition-colors {kindHl
							? 'bg-surface-hover'
							: ''}"
						onclick={() => toggleOpen(kKey)}
						onmouseenter={() => (highlightedKey = kKey)}
					>
						<ChevronRight
							size={10}
							class="transition-transform shrink-0 text-secondary {kindOpen ? 'rotate-90' : ''}"
						/>
						<RowIcon kind={k} size={12} />
						<span class="flex-1">{KIND_LABEL[k]}</span>
						{#if loadingKind[k]}
							<Loader2 size={12} class="animate-spin text-tertiary" />
						{/if}
					</button>
					{#if kindOpen}
						{#if !loaded[k] && !loadingKind[k]}
							<div class="px-8 py-1 text-2xs text-tertiary">Empty</div>
						{:else if loaded[k]?.length === 0}
							<div class="px-8 py-1 text-2xs text-tertiary">No {KIND_LABEL[k].toLowerCase()}</div>
						{:else if loaded[k]}
							{#each scopesByKind[k] as scope (scope)}
								{@const sKey = scopeKey(k, scope)}
								{@const scopeOpen = openItems.has(sKey)}
								{@const scopeHl = sKey === highlightedKey}
								<button
									type="button"
									data-nav-key={sKey}
									aria-expanded={scopeOpen}
									class="flex items-center gap-1.5 w-full text-left pl-7 pr-3 py-1.5 text-xs font-medium font-mono text-emphasis transition-colors {scopeHl
										? 'bg-surface-hover'
										: ''}"
									onclick={() => toggleOpen(sKey)}
									onmouseenter={() => (highlightedKey = sKey)}
								>
									<ChevronRight
										size={10}
										class="transition-transform shrink-0 text-secondary {scopeOpen
											? 'rotate-90'
											: ''}"
									/>
									{#if scope.startsWith('u/')}
										<User size={12} class="shrink-0 text-tertiary" />
									{:else}
										<Folder size={12} class="shrink-0 text-tertiary" />
									{/if}
									<span class="truncate">{scope}</span>
								</button>
								{#if scopeOpen}
									<ul>
										{#each groupedByScope[k].get(scope) ?? [] as it (it.path)}
											<li>
												{@render leafRow(it, it.path.slice(scope.length + 1), 'pl-12 pr-3 py-1.5')}
											</li>
										{/each}
									</ul>
								{/if}
							{/each}
						{/if}
					{/if}
				{/each}
			</div>
		{/if}
	</div>
</div>
