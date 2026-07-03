<script lang="ts" module>
	export type DiffStatus = 'added' | 'removed' | 'modified' | 'conflict'
	// One changed item. `status` drives the dot/badge; `ahead`/`behind` are
	// optional (fork-only) and only render when present.
	export type DiffRow = {
		kind: string
		path: string
		status: DiffStatus
		ahead?: number
		behind?: number
		/** Human-facing path; defaults to `path`. Lets a draft parked at a
		 * synthetic storage path (`…/draft_<uuid>`) show its friendly typed path
		 * while keys, value-loading and edit links stay keyed on `path`. */
		displayPath?: string
		/** Summary supplied by the data source. Preferred over the one derived
		 * from the loaded diff value, and shown before that value loads. */
		summary?: string
		/** Explicit unique row identity, overriding the default `kind/path`. For a
		 * row whose `kind/path` isn't unique on its own (a pipeline-bundle node
		 * shares `script/<path>` with a standalone script draft at the same path)
		 * while `path` must stay the real edit/display target. */
		key?: string
	}
</script>

<script lang="ts">
	import { buildDiffTree, type AppRootMeta, type TreeNode } from './diffTree'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import {
		AlertTriangle,
		ChevronDown,
		ChevronRight,
		Folder,
		GitMerge,
		Loader2,
		Minus,
		Pencil,
		Plus,
		User
	} from 'lucide-svelte'
	import { goto } from '$lib/navigation'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import WorkspaceItemRow from '$lib/components/WorkspaceItemRow.svelte'
	import WorkspaceItemDiffViewer from '$lib/components/WorkspaceItemDiffViewer.svelte'
	import {
		rawAppDiffToItems,
		type RawAppish,
		type RawAppFileItem,
		type RawAppRunnableItem
	} from '$lib/components/raw_apps/rawAppDiffUtils'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { DiffIcon, SquareSplitHorizontal } from 'lucide-svelte'
	import { untrack, type Snippet } from 'svelte'
	import ExternalEditLink from '../ExternalEditLink.svelte'

	// Read-only, multi-item before/after diff viewer with a file tree. The data
	// source (fork comparison vs deployed-vs-draft) is supplied by the parent
	// wrapper via `diffs` + `loadValues`; everything here is generic rendering.
	let {
		diffs,
		loadValues,
		loading = false,
		error = undefined,
		notice = undefined,
		emptyMessage = 'No changes.',
		title,
		reviewHref,
		reviewLabel = 'Review',
		editUrlFor = undefined,
		titleExtra
	}: {
		diffs: DiffRow[]
		loadValues: (d: DiffRow) => Promise<{ before: unknown; after: unknown }>
		loading?: boolean
		error?: string | undefined
		/** Replaces the diff list with an informational message (e.g. fork
		 * created before change tracking). */
		notice?: string | undefined
		emptyMessage?: string
		title: string
		reviewHref: string
		reviewLabel?: string
		editUrlFor?: (d: DiffRow) => string | undefined
		titleExtra?: Snippet
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let searchQuery = $state('')
	let diffStyle = $state<'sbs' | 'inline'>('sbs')
	const inlineDiff = $derived(diffStyle === 'inline')

	export function open() {
		// Reset per-item caches so an edit-then-reopen doesn't show stale
		// expanded content. The wrapper re-fetches `diffs`, which re-triggers
		// the eager-load effect below.
		loadedDiffs = {}
		summaries = {}
		mountedRows = {}
		drawer?.openDrawer()
		setTimeout(() => searchInputEl?.focus(), 50)
	}

	// A row in the list is either a real backend diff or a synthesized raw-app
	// item: a file/metadata leaf (`RawAppFileItem`) or a runnable rendered as a
	// script/flow (`RawAppRunnableItem`). All are diff-shaped, so the tree /
	// search / count / nav operate over `DisplayDiff` uniformly.
	type DisplayDiff = DiffRow | RawAppFileItem | RawAppRunnableItem

	function itemKey(d: DisplayDiff): string {
		// Synthetic raw-app items (files/runnables) carry their composite
		// `<appPath>/…` path and can share kind+path with a real workspace item —
		// e.g. a runnable rendered as `script` at `<appPath>/runnables/foo` vs a real
		// script literally at that path. Prefix synthetic items so the {#each} key,
		// load cache, row id and nav identity never collide with a real DiffRow.
		if ('key' in d && d.key) return d.key
		return ('appPath' in d ? 'rawapp:' : '') + `${d.kind}/${d.path}`
	}

	// Friendly path for display only; `path` stays the storage key everywhere
	// keys/loads happen, so a never-deployed draft still loads from `…/draft_<uuid>`.
	function displayPathOf(d: DisplayDiff): string {
		return ('displayPath' in d ? d.displayPath : undefined) ?? d.path
	}

	const KIND_LABELS: Record<string, string> = {
		script: 'Script',
		flow: 'Flow',
		app: 'App',
		raw_app: 'Raw app',
		raw_app_file: 'File',
		resource: 'Resource',
		variable: 'Variable',
		resource_type: 'Resource type',
		folder: 'Folder',
		schedule: 'Schedule',
		http_trigger: 'HTTP route',
		websocket_trigger: 'Websocket trigger',
		kafka_trigger: 'Kafka trigger',
		nats_trigger: 'NATS trigger',
		postgres_trigger: 'Postgres trigger',
		mqtt_trigger: 'MQTT trigger',
		sqs_trigger: 'SQS trigger',
		gcp_trigger: 'GCP trigger',
		azure_trigger: 'Azure trigger',
		email_trigger: 'Email trigger'
	}

	type LoadedDiff = {
		state: 'loading' | 'ready' | 'error'
		error?: string
		before?: unknown
		after?: unknown
	}
	let loadedDiffs: Record<string, LoadedDiff> = $state({})
	let summaries: Record<string, string | undefined> = $state({})

	// Lazy Monaco mount: exploding a raw app into N per-file rows would otherwise
	// mount N DiffEditors at once. Each row's editor mounts only once its block
	// scrolls within `MOUNT_MARGIN` of the viewport (rooted on the scroll
	// container); `mountedRows` latches true so it never unmounts on scroll-away.
	let mainScrollEl: HTMLElement | undefined = $state()
	let mountedRows: Record<string, boolean> = $state({})
	// Small look-ahead only: a large margin would catch the whole (collapsed-
	// height) list at once, and each mount growing its block would cascade the
	// rest into view. A modest margin mounts what's near the fold; as editors
	// grow they push the pending blocks away, so the rest stay deferred.
	const MOUNT_MARGIN = '200px 0px'
	function lazyMount(node: HTMLElement, key: string) {
		const io = new IntersectionObserver(
			(entries) => {
				if (entries.some((e) => e.isIntersecting)) {
					mountedRows[key] = true
					io.disconnect()
				}
			},
			{ root: mainScrollEl ?? null, rootMargin: MOUNT_MARGIN }
		)
		io.observe(node)
		return { destroy: () => io.disconnect() }
	}

	// The rendered list: each raw_app whose value has loaded is expanded into
	// its per-file items (so files are independent rows that flow through the
	// tree / search / count). An unloaded raw_app stays a single placeholder row
	// until its value arrives, then expands in place.
	const displayDiffs = $derived.by(() => {
		const out: DisplayDiff[] = []
		for (const d of diffs) {
			if (d.kind === 'raw_app') {
				const loaded = loadedDiffs[itemKey(d)]
				if (loaded?.state === 'ready') {
					out.push(
						...rawAppDiffToItems(
							d.path,
							loaded.before as RawAppish | undefined,
							loaded.after as RawAppish | undefined,
							displayPathOf(d)
						)
					)
					continue
				}
			}
			out.push(d)
		}
		return out
	})

	async function loadDiffFor(d: DiffRow) {
		const key = itemKey(d)
		if (loadedDiffs[key]) return
		loadedDiffs[key] = { state: 'loading' }
		try {
			const { before, after } = await loadValues(d)
			loadedDiffs[key] = { state: 'ready', before, after }
			const summary =
				(after && typeof after === 'object' && (after as any).summary) ||
				(before && typeof before === 'object' && (before as any).summary) ||
				undefined
			if (typeof summary === 'string' && summary.trim().length > 0) {
				summaries[key] = summary
			}
		} catch (e) {
			console.error('WorkspaceDiffDrawer: loadValues failed', d, e)
			loadedDiffs[key] = { state: 'error', error: String(e) }
		}
	}

	// Eagerly load each row (diffs render expanded). Tracks `diffs` only; the
	// cache reads/writes are untracked so this doesn't loop on itself.
	$effect(() => {
		const ds = diffs
		untrack(() => {
			for (const d of ds) void loadDiffFor(d)
		})
	})

	function onDetailsToggle(d: DisplayDiff, e: Event) {
		// Synthetic raw-app items (files + runnables) carry their content
		// already — nothing to fetch.
		if ('appPath' in d) return
		const target = e.currentTarget as HTMLDetailsElement | null
		if (target?.open) void loadDiffFor(d)
	}

	function statusBadgeColor(s: DiffStatus): 'green' | 'red' | 'orange' | 'blue' {
		if (s === 'added') return 'green'
		if (s === 'removed') return 'red'
		if (s === 'conflict') return 'orange'
		return 'blue'
	}

	const statusIcons = { added: Plus, removed: Minus, modified: Pencil, conflict: AlertTriangle }

	// ── File tree ───────────────────────────────────────────────────────────
	// Friendly app path → app metadata, for tagging the matching tree folder as a
	// raw-app root. Keyed on `displayPathOf` so it lines up with the friendly
	// virtual paths the synthetic file items carry.
	const appRootMeta = $derived.by(() => {
		const m = new Map<string, AppRootMeta>()
		for (const d of diffs) {
			if (d.kind !== 'raw_app') continue
			m.set(displayPathOf(d), {
				summaryKey: itemKey(d),
				summary: 'summary' in d ? d.summary : undefined
			})
		}
		return m
	})

	function searchableText(d: DisplayDiff): string {
		const parts = [displayPathOf(d), KIND_LABELS[d.kind] ?? d.kind]
		const s = summaries[itemKey(d)] ?? ('summary' in d ? d.summary : undefined)
		if (s) parts.push(s)
		return parts.join(' ')
	}
	let searchedDiffs: (DisplayDiff & { marked?: string })[] | undefined = $state(undefined)

	const filteredDiffs = $derived.by(() => {
		const q = searchQuery.trim()
		if (!q) return displayDiffs
		return (searchedDiffs ?? []) as DisplayDiff[]
	})

	const treeModel = $derived(
		buildDiffTree(
			filteredDiffs.map((d) => ({ key: itemKey(d), structurePath: displayPathOf(d), data: d })),
			appRootMeta
		)
	)

	function rowId(d: DisplayDiff): string {
		return `ws-diff-${itemKey(d)}`
	}

	function scrollToDiff(d: DisplayDiff) {
		const el = document.getElementById(rowId(d)) as HTMLDetailsElement | null
		if (!el) return
		el.open = true
		el.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	// ── Keyboard nav (matches WorkspaceItemDrillPicker) ─────────────────────
	let folderOpen: Record<string, boolean> = $state({})
	function isFolderOpen(key: string): boolean {
		return folderOpen[key] ?? true
	}

	const navEntries = $derived(treeModel.order((k) => isFolderOpen(k)))
	const navKeys = $derived(navEntries.map((e) => e.key))
	const entryByKey = $derived(new Map(navEntries.map((e) => [e.key, e])))

	let highlightedKey: string | undefined = $state(undefined)
	let mouseActive = $state(false)
	let searchInputEl: HTMLInputElement | undefined = $state()
	let sidebarRoot: HTMLElement | undefined = $state()

	$effect(() => {
		if (navKeys.length === 0) return
		if (!highlightedKey || !navKeys.includes(highlightedKey)) {
			highlightedKey = navKeys[0]
		}
	})

	function scrollHighlightIntoView() {
		if (!sidebarRoot || !highlightedKey) return
		const el = sidebarRoot.querySelector<HTMLElement>(
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
		if (mouseActive) highlightedKey = key
	}

	function activateHighlighted() {
		if (!highlightedKey) return
		const entry = entryByKey.get(highlightedKey)
		if (!entry) return
		if (entry.type === 'file') {
			scrollToDiff(entry.data)
		} else {
			folderOpen[entry.key] = !isFolderOpen(entry.key)
		}
	}

	function selectKey(key: string) {
		highlightedKey = key
		mouseActive = false
		requestAnimationFrame(scrollHighlightIntoView)
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault()
			moveHighlight(1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			moveHighlight(-1)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			activateHighlighted()
		} else if (e.key === 'ArrowRight') {
			const entry = highlightedKey ? entryByKey.get(highlightedKey) : undefined
			if (!entry || entry.type !== 'folder') return
			if (!isFolderOpen(entry.key)) {
				e.preventDefault()
				folderOpen[entry.key] = true
				return
			}
			const child = treeModel.firstChildKeyOf(entry.key)
			if (child) {
				e.preventDefault()
				selectKey(child)
			}
		} else if (e.key === 'ArrowLeft') {
			const entry = highlightedKey ? entryByKey.get(highlightedKey) : undefined
			if (!entry) return
			if (entry.type === 'folder' && isFolderOpen(entry.key)) {
				e.preventDefault()
				folderOpen[entry.key] = false
				return
			}
			const parent = treeModel.parentKeyOf(entry.key)
			if (parent && entryByKey.has(parent)) {
				e.preventDefault()
				selectKey(parent)
			}
		}
	}
</script>

<SearchItems
	filter={searchQuery}
	items={displayDiffs}
	bind:filteredItems={searchedDiffs}
	f={(d: DisplayDiff) => searchableText(d)}
/>

{#snippet renderTreeNode(node: TreeNode<DisplayDiff>, depth: number)}
	{#if node.type === 'folder'}
		{@const isUserScope = node.isScope && node.name.startsWith('u/')}
		{@const fkey = node.key}
		{@const open = isFolderOpen(fkey)}
		{@const isHl = fkey === highlightedKey}
		<details
			{open}
			ontoggle={(e) => (folderOpen[fkey] = (e.currentTarget as HTMLDetailsElement).open)}
			class="select-none"
		>
			{#if node.app}
				{@const appSummary = summaries[node.app.summaryKey] ?? node.app.summary}
				<!-- Raw-app root: expandable folder, but its header reads as a normal
				     item row (icon + label). Single line showing the summary, falling
				     back to the short name (the tree already places it under its scope;
				     full path on hover). The expand chevron sits at the end of the row. -->
				<summary
					role="option"
					aria-selected={isHl}
					data-nav-key={fkey}
					onmouseenter={() => setHoverHighlight(fkey)}
					title={node.fullPath}
					class="flex items-center gap-1.5 px-3 py-1.5 cursor-pointer hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
						? 'bg-surface-hover'
						: ''}"
					style="padding-left: {depth * 12 + 8}px"
				>
					<RowIcon kind="raw_app" size={12} />
					<span
						class="flex-1 min-w-0 truncate text-xs font-normal text-primary {appSummary
							? ''
							: 'font-mono'}"
					>
						{appSummary ?? node.name}
					</span>
					<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
					<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
				</summary>
			{:else}
				<summary
					role="option"
					aria-selected={isHl}
					data-nav-key={fkey}
					onmouseenter={() => setHoverHighlight(fkey)}
					class="flex items-center gap-1.5 px-3 py-1.5 cursor-pointer text-xs font-normal font-mono text-secondary hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
						? 'bg-surface-hover'
						: ''}"
					style="padding-left: {depth * 12 + 8}px"
				>
					{#if isUserScope}
						<User size={12} class="shrink-0 text-tertiary" />
					{:else}
						<Folder size={12} class="shrink-0 text-tertiary" />
					{/if}
					<span class="flex-1 min-w-0 truncate" title={node.name}>{node.name}</span>
					<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
					<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
				</summary>
			{/if}
			<div class="relative">
				<!-- Indent guide: a vertical rule down the left of the folder's
				     children, at the parent chevron's midpoint (depth*12+8 + 6).
				     Absolute so it doesn't shift the rows; clicks pass through. -->
				<div
					class="pointer-events-none absolute top-0 bottom-0 border-l border-light"
					style="left: {depth * 12 + 14}px"
					aria-hidden="true"
				></div>
				{#each node.children as child}
					{@render renderTreeNode(child, depth + 1)}
				{/each}
			</div>
		</details>
	{:else}
		{@const d = node.data}
		{@const key = node.key}
		<!-- Folders place their icon at the base padding (depth*12+8) — the chevron
		     now sits at the row's end, not the front. WorkspaceItemRow adds its own
		     12px base, so indent = depth*12-4 lines a file's icon up with sibling
		     folder icons. Keep in sync with the folder summary. -->
		<WorkspaceItemRow
			kind={d.kind as any}
			iconPath={d.path}
			baseClass="py-1.5"
			singleLine
			summary={summaries[key] ?? ('summary' in d ? d.summary : undefined)}
			secondary={node.name}
			highlighted={key === highlightedKey}
			navKey={key}
			indent={depth * 12 - 4}
			title={displayPathOf(d)}
			onclick={() => {
				highlightedKey = key
				scrollToDiff(d)
			}}
			onmouseenter={() => setHoverHighlight(key)}
		/>
	{/if}
{/snippet}

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		{title}
		{titleExtra}
		on:close={() => drawer?.closeDrawer()}
		documentationLink={undefined}
		noPadding
		overflow_y={false}
	>
		{#snippet actions()}
			<ToggleButtonGroup bind:selected={diffStyle} noWFull>
				{#snippet children({ item })}
					<ToggleButton
						value="sbs"
						label="Side-by-side"
						icon={SquareSplitHorizontal}
						tooltip="Side-by-side diff"
						iconOnly
						{item}
					/>
					<ToggleButton
						value="inline"
						label="Unified"
						icon={DiffIcon}
						tooltip="Unified diff"
						iconOnly
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: GitMerge }}
				onclick={() => goto(reviewHref)}
			>
				{reviewLabel}
			</Button>
		{/snippet}
		<div class="flex flex-row h-full min-h-0">
			{#if diffs.length > 0}
				<aside
					bind:this={sidebarRoot}
					onmousemove={() => (mouseActive = true)}
					class="flex-none w-56 border-r border-light flex flex-col min-h-0"
				>
					<div class="px-3 pt-3 pb-2 shrink-0">
						<!-- Raw input (not the design-system TextInput) on purpose: this is a
						     bespoke filter wired to the file tree's keyboard navigation — it
						     needs a direct element ref to focus (the `/` shortcut) and a
						     keydown handler that hands ArrowDown/Enter off to the tree.
						     TextInput/ClearableInput swallow/rebubble those and don't expose
						     the element ref. -->
						<input
							bind:this={searchInputEl}
							type="search"
							bind:value={searchQuery}
							placeholder="Filter files..."
							onkeydown={handleSearchKeydown}
							class="w-full text-xs px-2 py-1 rounded border border-light bg-surface focus:outline-none focus:border-accent"
						/>
					</div>
					<div class="flex-1 min-h-0 overflow-y-auto pb-3 flex flex-col gap-1">
						{#if treeModel.root.children.length > 0}
							{#each treeModel.root.children as child}
								{@render renderTreeNode(child, 0)}
							{/each}
						{:else}
							<div class="text-2xs text-tertiary px-3 py-2">No matches</div>
						{/if}
					</div>
				</aside>
			{/if}
			<main bind:this={mainScrollEl} class="flex-1 min-w-0 overflow-y-auto">
				<div class="px-3 pt-3 pb-4 flex flex-col gap-3">
					{#if loading && diffs.length === 0}
						<div class="flex items-center gap-2 text-sm text-secondary py-8 self-center">
							<Loader2 class="w-4 h-4 animate-spin" />
							Loading comparison...
						</div>
					{:else if error}
						<div class="text-sm text-red-600 dark:text-red-400 py-4">{error}</div>
					{:else if notice}
						<div class="text-sm text-secondary py-4">{notice}</div>
					{:else if diffs.length === 0}
						<div class="text-sm text-secondary py-4">{emptyMessage}</div>
					{:else if filteredDiffs.length === 0}
						<div class="text-sm text-secondary py-4">No files match "{searchQuery}".</div>
					{:else}
						<div class="flex flex-col gap-2">
							{#each filteredDiffs as d (itemKey(d))}
								{@const key = itemKey(d)}
								{@const status = d.status}
								{@const StatusIcon = statusIcons[status]}
								{@const loaded = loadedDiffs[key]}
								<!-- A synthesized raw-app item (file or runnable) links to its
								     owning app's editor. -->
								{@const editUrl =
									'appPath' in d
										? editUrlFor?.({ ...d, kind: 'raw_app', path: d.appPath })
										: editUrlFor?.(d)}
								{@const dpath = displayPathOf(d)}
								<details
									open
									id={rowId(d)}
									class="border border-light rounded-md bg-surface scroll-mt-2"
									ontoggle={(e) => onDetailsToggle(d, e)}
								>
									<summary
										class="sticky top-0 z-30 bg-surface flex items-center gap-2 px-3 py-2 cursor-pointer list-none [&::-webkit-details-marker]:hidden border-b border-transparent rounded-md relative before:content-[''] before:absolute before:inset-0 before:bg-surface-hover before:opacity-0 before:pointer-events-none before:transition-opacity hover:before:opacity-100"
									>
										<ChevronDown
											class="w-3.5 h-3.5 shrink-0 text-tertiary transition-transform chevron"
										/>
										<RowIcon kind={d.kind as any} path={d.path} size={14} />
										<div class="min-w-0 flex-1">
											{#if editUrl}
												<ExternalEditLink
													href={editUrl}
													title={dpath}
													class="text-xs text-primary font-mono truncate"
												>
													<span class="truncate">{dpath}</span>
												</ExternalEditLink>
											{:else}
												<div class="text-xs text-primary font-mono truncate" title={dpath}>
													{dpath}
												</div>
											{/if}
										</div>
										<div class="shrink-0 flex items-center gap-2">
											{#if d.ahead && d.ahead > 0}
												<span class="text-2xs text-secondary">{d.ahead} ahead</span>
											{/if}
											{#if d.behind && d.behind > 0}
												<span class="text-2xs text-secondary">{d.behind} behind</span>
											{/if}
											<Badge color={statusBadgeColor(status)}>
												<StatusIcon class="w-3 h-3 inline mr-0.5" />
												{status}
											</Badge>
										</div>
									</summary>
									<div
										class="border-t border-light bg-surface-tertiary rounded-b-md overflow-hidden"
									>
										{#if !mountedRows[key]}
											<!-- Defer the Monaco mount until this block scrolls near the
											     viewport, so an exploded many-file app doesn't mount every
											     editor at once. -->
											<div
												use:lazyMount={key}
												class="flex items-center gap-2 text-2xs text-tertiary p-3 min-h-[10rem]"
											>
												<Loader2 class="w-3.5 h-3.5 animate-spin" />
												Diff loads on scroll…
											</div>
										{:else if d.kind === 'raw_app_file'}
											<!-- DiffRow.kind is a plain string, so the kind check doesn't narrow
											     the union — assert the synthetic file item. -->
											{@const rawFile = d as RawAppFileItem}
											<WorkspaceItemDiffViewer kind="raw_app_file" {rawFile} {inlineDiff} />
										{:else if 'appPath' in d}
											<!-- Synthesized runnable: render script-style (Content + Metadata),
											     forcing `script` so flow runnables don't hit FlowDiffViewer. -->
											{@const runnable = d as RawAppRunnableItem}
											<WorkspaceItemDiffViewer
												kind="script"
												originalRaw={runnable.originalRaw}
												currentRaw={runnable.currentRaw}
												{inlineDiff}
											/>
										{:else if !loaded || loaded.state === 'loading'}
											<div class="flex items-center gap-2 text-xs text-secondary p-3">
												<Loader2 class="w-3.5 h-3.5 animate-spin" />
												Loading diff…
											</div>
										{:else if loaded.state === 'error'}
											<div class="text-xs text-red-600 dark:text-red-400">{loaded.error}</div>
										{:else if loaded.state === 'ready'}
											<WorkspaceItemDiffViewer
												kind={d.kind}
												originalRaw={loaded.before}
												currentRaw={loaded.after}
												{inlineDiff}
											/>
										{/if}
									</div>
								</details>
							{/each}
						</div>
					{/if}
				</div></main
			>
		</div>
	</DrawerContent>
</Drawer>

<style>
	details:not([open]) :global(.chevron) {
		transform: rotate(-90deg);
	}
	details:not([open]) > .tree-summary :global(.tree-chevron-open) {
		display: none;
	}
	details[open] > .tree-summary :global(.tree-chevron-closed) {
		display: none;
	}
</style>
