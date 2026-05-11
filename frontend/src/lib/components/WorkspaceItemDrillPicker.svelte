<!--
@component
Drill-through workspace item picker. One level is shown at a time:

  - **Root** (no scope): All + kinds (Flows / Scripts / Apps). "All" is a
    cross-kind row — drilling in shows folders/items merged across every kind.
  - **Kind** (`{ kind }`): top-level scopes for that kind (e.g. `f/demo`, `u/alice`).
    `kind: 'all'` is the cross-kind variant — folders contain items from
    every kind, leaves still belong to a real kind.
  - **Dir** (`{ kind, dir }`): immediate children of `dir` — subdirs + leaves.

Clicking a row drills *down*; the chevron-left in the header walks one level
*up*. Search is global across all kinds and ignores the current scope.
-->
<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { ChevronLeft, ChevronRight, Folder, Layers, Loader2, User } from 'lucide-svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { onMount, untrack } from 'svelte'
	import {
		dirKey,
		getCachedItems,
		KIND_LABEL,
		KIND_LABEL_LOWER,
		kindKey,
		leafKeyFor,
		loadKind,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from './workspacePicker'

	type Kind = WorkspaceItemKind
	type Item = WorkspaceItem
	/** `'all'` is a virtual cross-kind scope — items still belong to a real
	 * kind, but folders and the root row group items from every kind. */
	type ScopeKind = Kind | 'all'

	export type Scope = { kind: ScopeKind; dir?: string } | undefined

	interface Props {
		onPick: (item: WorkspaceItem) => void
		kinds?: Kind[]
		/** Where the picker lands when first opened. `undefined` = root (kinds list). */
		initialScope?: Scope
		/** Composite key of the row to highlight (e.g. `dir:flow:f/demo`). */
		initialHighlight?: string
		/** Currently-edited item — gets `aria-current` and a no-op click. If
		 * `savedPath` differs from `path` (draft rename), the saved entry is
		 * suppressed so only the live one shows. */
		currentItem?: WorkspaceItem & { savedPath?: string }
	}

	let {
		onPick,
		kinds = ['flow', 'script', 'app'],
		initialScope,
		initialHighlight,
		currentItem
	}: Props = $props()

	let searchInput: TextInput | undefined = $state()
	let pickerRoot: HTMLElement | undefined = $state()
	const instanceId = crypto.randomUUID()
	const listboxId = `pkr-list-${instanceId}`
	const idFor = (key: string) => `pkr-${instanceId}-${key.replace(/[^a-zA-Z0-9-]/g, '_')}`

	export function focus() {
		searchInput?.focus()
	}

	// Sibling-popover open: melt-ui's `openFocus` runs once during the close→open
	// transition; the picker may not be mounted yet. Retry after settle.
	onMount(() => {
		const t = setTimeout(focus, 50)
		return () => clearTimeout(t)
	})

	const leafKey = (it: Item) => leafKeyFor(it.kind, it.path)

	let scope = $state<Scope>(untrack(() => initialScope))
	let filter = $state('')

	/** Tracks whether the last user action was mouse movement (true) or
	 * keyboard nav (false). When false, row `mouseenter` events are ignored
	 * — prevents the cursor from stealing the keyboard-driven highlight as
	 * rows shift under it during scope changes. Re-enabled on `mousemove`.
	 * Starts `false` so the synthetic `mouseenter` fired when the popover
	 * mounts under a stationary cursor doesn't clobber `initialHighlight`. */
	let mouseActive = $state(false)

	// Seed from cache so kinds already fetched in this session render on the
	// first frame. Read once at mount: melt-ui mounts a fresh picker per
	// popover open, so workspace changes are picked up at the next open
	// without needing this seed to be reactive.
	let loaded = $state<Partial<Record<Kind, Item[]>>>(
		(() => {
			if (!$workspaceStore) return {}
			const out: Partial<Record<Kind, Item[]>> = {}
			for (const k of kinds) {
				const cached = getCachedItems($workspaceStore, k)
				if (cached) out[k] = cached
			}
			return out
		})()
	)
	let loadingKind = $state<Partial<Record<Kind, boolean>>>({})

	async function ensureLoaded(kind: Kind) {
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

	// Fetch the scope's kind on entry to a non-root level. The `'all'` scope
	// needs every kind loaded since it merges items across them.
	$effect(() => {
		if (!scope) return
		if (scope.kind === 'all') for (const k of kinds) ensureLoaded(k)
		else ensureLoaded(scope.kind)
	})

	// Searching is global → load every kind.
	$effect(() => {
		if (filter.trim() !== '') for (const k of kinds) ensureLoaded(k)
	})

	type DirNode = {
		fullPath: string
		name: string
		isScope: boolean
		children: DirNode[]
		leaves: Item[]
	}

	/** Inject the currently-edited item into a kind's list at its live path,
	 * dropping the saved entry when a draft rename is in progress. Other kinds
	 * pass through untouched. */
	function withCurrent(items: Item[], k: Kind): Item[] {
		if (!currentItem || currentItem.kind !== k) return items
		const drafted =
			currentItem.savedPath && currentItem.savedPath !== currentItem.path
				? items.filter((it) => it.path !== currentItem.savedPath)
				: items
		if (drafted.some((it) => it.path === currentItem.path)) return drafted
		return [
			...drafted,
			{
				path: currentItem.path,
				summary: currentItem.summary,
				kind: k,
				raw_app: currentItem.raw_app
			}
		]
	}

	function buildTreeFromItems(items: Item[]): DirNode[] {
		const scopeRoots = new Map<string, DirNode>()
		for (const it of items) {
			const parts = it.path.split('/')
			if (parts.length < 3) continue
			const scopeFp = parts.slice(0, 2).join('/')
			let node = scopeRoots.get(scopeFp)
			if (!node) {
				node = { fullPath: scopeFp, name: scopeFp, isScope: true, children: [], leaves: [] }
				scopeRoots.set(scopeFp, node)
			}
			const slug = parts.slice(2)
			let cur = node
			for (let i = 0; i < slug.length - 1; i++) {
				const seg = slug[i]
				const fullPath = cur.fullPath + '/' + seg
				let next = cur.children.find((c) => c.name === seg)
				if (!next) {
					next = { fullPath, name: seg, isScope: false, children: [], leaves: [] }
					cur.children.push(next)
				}
				cur = next
			}
			cur.leaves.push(it)
		}
		const scopes = Array.from(scopeRoots.values()).sort((a, b) => {
			const af = a.fullPath.startsWith('f/') ? 0 : 1
			const bf = b.fullPath.startsWith('f/') ? 0 : 1
			if (af !== bf) return af - bf
			return a.fullPath.localeCompare(b.fullPath)
		})
		const sortNode = (n: DirNode) => {
			n.children.sort((a, b) => a.name.localeCompare(b.name))
			n.leaves.sort((a, b) => a.path.localeCompare(b.path))
			n.children.forEach(sortNode)
		}
		scopes.forEach(sortNode)
		return scopes
	}

	/** Per-kind tree deriveds. Each only re-evaluates `buildTreeFromItems`
	 * when its own `loaded[k]` changes or when the user is mid-rename on
	 * that kind — typing in a flow's path edit leaves script/app trees
	 * cached. */
	function buildIfActive(k: Kind, list: Item[] | undefined): DirNode[] {
		if (!kinds.includes(k)) return []
		const items = withCurrent(list ?? [], k)
		if (items.length === 0) return []
		return buildTreeFromItems(items)
	}

	const flowTree = $derived(buildIfActive('flow', loaded.flow))
	const scriptTree = $derived(buildIfActive('script', loaded.script))
	const appTree = $derived(buildIfActive('app', loaded.app))
	/** Cross-kind tree: every loaded item from every active kind, merged into
	 * one folder hierarchy. Each leaf still carries its real kind, so the row
	 * icon and `editPathFor` routing still work; folders contain a mix. */
	const allTree = $derived.by(() => {
		const merged = kinds.flatMap((k) => withCurrent(loaded[k] ?? [], k))
		return merged.length === 0 ? [] : buildTreeFromItems(merged)
	})

	function treeFor(k: ScopeKind): DirNode[] {
		if (k === 'all') return allTree
		if (k === 'flow') return flowTree
		if (k === 'script') return scriptTree
		return appTree
	}

	function findDirInList(list: DirNode[], fullPath: string): DirNode | undefined {
		for (const n of list) {
			if (n.fullPath === fullPath) return n
			const sub = findDirInList(n.children, fullPath)
			if (sub) return sub
		}
		return undefined
	}

	function parentDirPath(p: string): string | undefined {
		const parts = p.split('/')
		if (parts.length <= 2) return undefined
		return parts.slice(0, -1).join('/')
	}

	type Entry =
		| { type: 'kind'; key: string; kind: ScopeKind }
		| { type: 'dir'; key: string; kind: ScopeKind; node: DirNode }
		| { type: 'leaf'; key: string; item: Item }

	type DisplayItem = Item & { marked?: string }
	type SearchInput = Item & { _key: string }

	let allItems = $derived<SearchInput[]>(
		kinds.flatMap((k) =>
			withCurrent(loaded[k] ?? [], k).map((it) => ({ ...it, _key: `${k}:${it.path}` }))
		)
	)

	let searchedItems: DisplayItem[] | undefined = $state(undefined)

	let isSearching = $derived(filter.trim() !== '')

	let searchResultsByKind = $derived.by(() => {
		const out: Record<Kind, DisplayItem[]> = { flow: [], script: [], app: [] }
		if (!searchedItems) return out
		for (const it of searchedItems) out[it.kind].push(it)
		return out
	})

	/** Rows currently shown — drives both rendering and keyboard nav. Not used
	 * while `isSearching` (search renders its own grouped layout). */
	let entries = $derived.by<Entry[]>(() => {
		const s = scope
		if (!s) {
			const kindEntries = kinds.map((k) => ({
				type: 'kind' as const,
				key: kindKey(k),
				kind: k
			}))
			// "All" only makes sense across multiple kinds — with a single kind
			// it would duplicate that kind's own root row.
			if (kinds.length <= 1) return kindEntries
			return [
				{ type: 'kind' as const, key: kindKey('all'), kind: 'all' as ScopeKind },
				...kindEntries
			]
		}
		const tree = treeFor(s.kind)
		if (!s.dir) {
			return tree.map((node) => ({
				type: 'dir',
				key: dirKey(s.kind, node.fullPath),
				kind: s.kind,
				node
			}))
		}
		const node = findDirInList(tree, s.dir)
		if (!node) return []
		return [
			...node.children.map(
				(c): Entry => ({
					type: 'dir',
					key: dirKey(s.kind, c.fullPath),
					kind: s.kind,
					node: c
				})
			),
			...node.leaves.map((l): Entry => ({ type: 'leaf', key: leafKey(l), item: l }))
		]
	})

	let navKeys = $derived.by(() => {
		if (isSearching) {
			return kinds.flatMap((k) => searchResultsByKind[k].map((it) => leafKey(it)))
		}
		return entries.map((e) => e.key)
	})

	let highlightedKey = $state<string | undefined>(untrack(() => initialHighlight))
	let highlightedId = $derived(highlightedKey ? idFor(highlightedKey) : undefined)

	$effect(() => {
		if (navKeys.length === 0) return
		if (!highlightedKey || !navKeys.includes(highlightedKey)) {
			highlightedKey = navKeys[0]
		}
	})

	$effect(() => {
		if (highlightedKey && navKeys.includes(highlightedKey)) {
			requestAnimationFrame(scrollHighlightIntoView)
		}
	})

	function scrollHighlightIntoView() {
		if (!pickerRoot || !highlightedKey) return
		const el = pickerRoot.querySelector<HTMLElement>(
			`[data-nav-key="${CSS.escape(highlightedKey)}"]`
		)
		el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
	}

	function moveHighlight(delta: 1 | -1) {
		if (navKeys.length === 0) return
		const cur = navKeys.indexOf(highlightedKey ?? '')
		const next = cur < 0 ? 0 : (cur + delta + navKeys.length) % navKeys.length
		highlightedKey = navKeys[next]
		mouseActive = false
		requestAnimationFrame(scrollHighlightIntoView)
	}

	function setHoverHighlight(key: string) {
		// Ignored until the user actually moves the mouse. Prevents the cursor
		// (parked over a row) from clobbering keyboard-driven selection when
		// the layout shifts beneath it.
		if (mouseActive) highlightedKey = key
	}

	function isCurrent(it: Item): boolean {
		return !!currentItem && currentItem.kind === it.kind && currentItem.path === it.path
	}

	function pick(it: Item) {
		if (isCurrent(it)) return
		onPick({ path: it.path, summary: it.summary, kind: it.kind, raw_app: it.raw_app })
	}

	function activate(key: string | undefined) {
		if (!key) return
		if (isSearching) {
			const flat = kinds.flatMap((k) => searchResultsByKind[k])
			const it = flat.find((x) => leafKey(x) === key)
			if (it) pick(it)
			return
		}
		const entry = entries.find((e) => e.key === key)
		if (!entry) return
		drill(entry)
	}

	function drill(entry: Entry) {
		if (entry.type === 'kind') {
			scope = { kind: entry.kind }
		} else if (entry.type === 'dir') {
			scope = { kind: entry.kind, dir: entry.node.fullPath }
		} else {
			pick(entry.item)
		}
	}

	function goUp() {
		if (!scope) return
		// Highlight the row in the parent view that represents the scope we
		// just left, so the user sees where they came from.
		if (!scope.dir) {
			const leaving = kindKey(scope.kind)
			scope = undefined
			highlightedKey = leaving
			return
		}
		const leaving = dirKey(scope.kind, scope.dir)
		const parent = parentDirPath(scope.dir)
		scope = parent ? { kind: scope.kind, dir: parent } : { kind: scope.kind }
		highlightedKey = leaving
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
			mouseActive = false
			activate(highlightedKey)
		} else if ((e.key === 'ArrowLeft' || e.key === 'Backspace') && filter === '' && scope) {
			// Walk up the tree. Only when search is empty — otherwise these
			// keys would hijack cursor movement / character deletion.
			e.preventDefault()
			e.stopPropagation()
			mouseActive = false
			goUp()
		} else if (e.key === 'ArrowRight' && filter === '' && !isSearching) {
			// Drill into the highlighted folder/kind. Leaves are reserved for
			// Enter (more deliberate, since picking navigates away).
			const entry = entries.find((en) => en.key === highlightedKey)
			if (entry && entry.type !== 'leaf') {
				e.preventDefault()
				e.stopPropagation()
				mouseActive = false
				drill(entry)
			}
		}
	}

	/** Breadcrumb segments for the header — kind name first, then the scope
	 * (`f/<folder>` or `u/<user>`) as one chunk, then any nested subdirs. */
	let headerSegments = $derived.by<string[]>(() => {
		if (!scope) return []
		const out = [scope.kind === 'all' ? 'all' : KIND_LABEL_LOWER[scope.kind]]
		if (scope.dir) {
			const parts = scope.dir.split('/')
			out.push(parts.slice(0, 2).join('/'))
			for (let i = 2; i < parts.length; i++) out.push(parts[i])
		}
		return out
	})

	/** Full breadcrumb (used for the hover tooltip). */
	let headerLabel = $derived(headerSegments.join(' › '))

	/** Number of intermediate segments to hide behind a `…`. The collapsed
	 * window is segments[2 .. 2 + hiddenCount); we always keep the first
	 * two (kind, top-level scope) and the deepest segment. Bumped up by an
	 * effect that measures actual overflow — see below. */
	let hiddenCount = $state(0)

	/** Breadcrumb shown to the user. Hides intermediate segments first, then
	 * relies on `truncate-start` for any remaining overflow on the deepest
	 * segment. Hover reveals the full path via `title`. */
	let headerLabelDisplay = $derived.by(() => {
		if (headerSegments.length <= 3 || hiddenCount === 0) return headerLabel
		return [
			headerSegments[0],
			headerSegments[1],
			'…',
			...headerSegments.slice(2 + hiddenCount)
		].join(' › ')
	})

	let breadcrumbSpan: HTMLElement | undefined = $state()
	let lastSegmentsKey = ''

	/** Measurement loop: each pass reads `scrollWidth > clientWidth` on the
	 * truncated span; if overflowing and there's still an intermediate segment
	 * to drop, increment `hiddenCount`. Mutating `hiddenCount` re-renders and
	 * re-fires this effect, so the loop self-terminates either when the text
	 * fits or when only [kind, scope, …, leaf] remain (`truncate-start` then
	 * polishes any final overflow). When the breadcrumb itself changes (new
	 * scope), reset to 0 first so a shorter path can re-expand. */
	$effect(() => {
		const key = headerSegments.join('|')
		const segmentsChanged = key !== lastSegmentsKey
		if (segmentsChanged) {
			lastSegmentsKey = key
			if (hiddenCount !== 0) {
				hiddenCount = 0
				return
			}
		}
		// Track hiddenCount so each collapse step re-measures.
		void hiddenCount
		if (!breadcrumbSpan) return
		const maxHide = Math.max(0, headerSegments.length - 3)
		if (hiddenCount >= maxHide) return
		queueMicrotask(() => {
			if (!breadcrumbSpan) return
			if (breadcrumbSpan.scrollWidth > breadcrumbSpan.clientWidth + 1) {
				hiddenCount = hiddenCount + 1
			}
		})
	})

	let scopeLoading = $derived.by(() => {
		if (!scope) return false
		if (scope.kind === 'all') {
			return kinds.some((k) => !loaded[k] && !!loadingKind[k])
		}
		return !loaded[scope.kind] && !!loadingKind[scope.kind]
	})
</script>

<SearchItems
	{filter}
	items={isSearching ? allItems : []}
	bind:filteredItems={searchedItems}
	f={(x: SearchInput) => (x.summary ? `${x.summary} (${x.path})` : x.path)}
	opts={{}}
/>

{#snippet leafRow(it: Item, secondary: string, baseClass: string)}
	{@const key = leafKey(it)}
	{@const isHl = key === highlightedKey}
	{@const isCur = isCurrent(it)}
	<button
		type="button"
		id={idFor(key)}
		role="option"
		aria-selected={isHl}
		data-nav-key={key}
		aria-current={isCur ? 'true' : undefined}
		class="w-full text-left flex items-center gap-2 px-3 transition-colors {baseClass} {isHl
			? 'bg-surface-hover'
			: ''} {isCur ? 'cursor-default text-emphasis font-medium' : ''}"
		onmousedown={(e) => e.preventDefault()}
		onclick={() => pick(it)}
		onmouseenter={() => setHoverHighlight(key)}
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
	onmousemove={() => (mouseActive = true)}
>
	<div class="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
		<TextInput
			bind:this={searchInput}
			bind:value={filter}
			size="sm"
			inputProps={{
				placeholder: 'Search by name or summary...',
				'data-workspace-picker-search': '',
				role: 'combobox',
				'aria-controls': listboxId,
				'aria-expanded': 'true',
				'aria-autocomplete': 'list',
				'aria-activedescendant': highlightedId
			}}
		/>
	</div>

	{#if scope}
		{@const s = scope}
		<button
			type="button"
			class="flex items-center gap-1.5 w-full text-left px-3 py-1 text-xs font-medium font-mono text-secondary bg-surface-secondary/20 hover:bg-surface-hover transition-colors"
			onmousedown={(e) => e.preventDefault()}
			onclick={goUp}
			title={headerLabel}
		>
			<ChevronLeft size={12} class="shrink-0 text-secondary" />
			{#if s.kind === 'all'}
				<Layers size={12} class="shrink-0 text-tertiary" />
			{:else}
				<RowIcon kind={s.kind} size={12} />
			{/if}
			<span bind:this={breadcrumbSpan} class="flex-1 min-w-0 truncate truncate-start"
				>{headerLabelDisplay}</span
			>
		</button>
	{/if}

	<div class="flex-1 overflow-y-auto" role="listbox" id={listboxId}>
		{#if isSearching}
			{@const total = (searchedItems ?? []).length}
			{@const anyKindLoading = kinds.some((k) => loadingKind[k])}
			{#if !searchedItems || anyKindLoading}
				<!-- "Searching…" while any active kind is still loading, otherwise
				     `SearchItems` would briefly write `filteredItems=[]` from the
				     partial set and flash "No matches" before results trickle in. -->
				<div role="status" class="px-3 py-2 text-xs text-tertiary flex items-center gap-2">
					<Loader2 size={14} class="animate-spin" /> Searching…
				</div>
			{:else if total === 0}
				<div role="status" class="px-3 py-2 text-xs text-tertiary">No matches</div>
			{:else}
				{#each kinds as k (k)}
					{@const results = searchResultsByKind[k]}
					{#if results.length > 0}
						<div class="px-3 pt-3 pb-1 text-2xs uppercase tracking-wide text-tertiary font-medium">
							{KIND_LABEL[k]}
						</div>
						<ul class="pb-1">
							{#each results as it (leafKey(it))}
								<li>{@render leafRow(it, it.path, 'py-1.5')}</li>
							{/each}
						</ul>
					{/if}
				{/each}
			{/if}
		{:else if scopeLoading && entries.length === 0}
			<div role="status" class="px-3 py-2 text-xs text-tertiary flex items-center gap-2">
				<Loader2 size={14} class="animate-spin" /> Loading…
			</div>
		{:else if entries.length === 0}
			<div role="status" class="px-3 py-2 text-xs text-tertiary">Empty</div>
		{:else}
			<div class="flex flex-col py-1">
				{#each entries as entry (entry.key)}
					{@const isHl = entry.key === highlightedKey}
					{#if entry.type === 'leaf'}
						{@render leafRow(
							entry.item,
							scope?.dir ? entry.item.path.slice(scope.dir.length + 1) : entry.item.path,
							'py-1.5'
						)}
					{:else}
						<button
							type="button"
							id={idFor(entry.key)}
							role="option"
							aria-selected={isHl}
							data-nav-key={entry.key}
							class="flex items-center gap-1.5 w-full text-left px-3 py-1.5 text-xs font-medium font-mono text-emphasis transition-colors {isHl
								? 'bg-surface-hover'
								: ''}"
							onmousedown={(e) => e.preventDefault()}
							onclick={() => drill(entry)}
							onmouseenter={() => setHoverHighlight(entry.key)}
						>
							{#if entry.type === 'kind'}
								{#if entry.kind === 'all'}
									<Layers size={12} class="shrink-0 text-tertiary" />
									<span class="flex-1">All</span>
								{:else}
									<RowIcon kind={entry.kind} size={12} />
									<span class="flex-1">{KIND_LABEL[entry.kind]}</span>
								{/if}
							{:else if entry.node.isScope && entry.node.fullPath.startsWith('u/')}
								<User size={12} class="shrink-0 text-tertiary" />
								<span class="flex-1 truncate">{entry.node.name}</span>
							{:else}
								<Folder size={12} class="shrink-0 text-tertiary" />
								<span class="flex-1 truncate">{entry.node.name}</span>
							{/if}
							{#if entry.type === 'kind' && entry.kind !== 'all' && loadingKind[entry.kind]}
								<Loader2 size={12} class="animate-spin text-tertiary" />
							{/if}
							<ChevronRight size={10} class="shrink-0 text-secondary" />
						</button>
					{/if}
				{/each}
			</div>
		{/if}
	</div>
</div>

<style>
	/* Path truncates from the start (left ellipsis) so the deepest (rightmost)
	 * folder stays visible. `unicode-bidi: plaintext` keeps each path segment
	 * laid out per its own direction — defends against any future RTL char
	 * appearing in a workspace path. */
	.truncate-start {
		direction: rtl;
		text-align: left;
		unicode-bidi: plaintext;
	}
</style>
