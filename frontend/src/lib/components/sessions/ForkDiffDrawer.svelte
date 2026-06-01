<script lang="ts">
	import { parentFolderKey } from './forkDiffNav'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import {
		AlertTriangle,
		ArrowRight,
		ChevronDown,
		ChevronRight,
		Folder,
		GitFork,
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
	import SearchItems from '$lib/components/SearchItems.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { DiffIcon, ExternalLink, SquareSplitHorizontal } from 'lucide-svelte'
	import { WorkspaceService, type WorkspaceComparison, type WorkspaceItemDiff } from '$lib/gen'
	import { getItemValue } from '$lib/utils_workspace_deploy'
	import { userWorkspaces } from '$lib/stores'
	import { editUrlFor as buildEditUrl } from './forkEditUrl'

	let {
		forkWorkspaceId,
		parentWorkspaceId
	}: { forkWorkspaceId: string; parentWorkspaceId: string } = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let comparison: WorkspaceComparison | undefined = $state(undefined)
	let loading = $state(false)
	let error: string | undefined = $state(undefined)
	let searchQuery = $state('')
	let diffStyle = $state<'sbs' | 'inline'>('sbs')
	const inlineDiff = $derived(diffStyle === 'inline')

	const forkWs = $derived($userWorkspaces.find((w) => w.id === forkWorkspaceId))
	const parentWs = $derived($userWorkspaces.find((w) => w.id === parentWorkspaceId))

	export function open() {
		drawer?.openDrawer()
		void fetchComparison()
		// Pull focus into the filter input so keyboard nav works without an
		// extra click — drawer transition needs a tick first.
		setTimeout(() => searchInputEl?.focus(), 50)
	}

	function openReview() {
		goto(`/forks/compare?workspace_id=${encodeURIComponent(forkWorkspaceId)}`)
	}

	async function fetchComparison() {
		loading = true
		error = undefined
		// Per-item raw diffs are cached for the lifetime of the drawer.
		// `loadDiffFor` early-returns on cache hit, so without this reset an
		// edit-then-reopen would show fresh summary/counts but stale expanded
		// raw content for any item the user had already drilled into.
		loadedDiffs = {}
		summaries = {}
		try {
			comparison = await WorkspaceService.compareWorkspaces({
				workspace: parentWorkspaceId,
				targetWorkspaceId: forkWorkspaceId
			})
			// Diffs are expanded by default, so eagerly populate each row's
			// content. Each loadDiffFor is idempotent and per-item, so
			// rendering proceeds as values arrive.
			if (comparison) {
				for (const d of comparison.diffs) {
					void loadDiffFor(d)
				}
			}
		} catch (e) {
			console.error('Fork diff: comparison failed', e)
			error = `Failed to load comparison: ${e}`
			comparison = undefined
		} finally {
			loading = false
		}
	}

	type DiffStatus = 'added' | 'removed' | 'modified' | 'conflict'

	function statusOf(d: WorkspaceItemDiff): DiffStatus {
		if (d.exists_in_fork && !d.exists_in_source) return 'added'
		if (!d.exists_in_fork && d.exists_in_source) return 'removed'
		if (d.ahead > 0 && d.behind > 0) return 'conflict'
		return 'modified'
	}

	function itemKey(d: WorkspaceItemDiff): string {
		return `${d.kind}/${d.path}`
	}

	// Editor URL for a diff row, scoped to the fork workspace.
	function editUrlFor(d: WorkspaceItemDiff): string | undefined {
		return buildEditUrl(d, forkWorkspaceId)
	}

	const KIND_LABELS: Record<string, string> = {
		script: 'Script',
		flow: 'Flow',
		app: 'App',
		raw_app: 'Raw app',
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

	// Lazily-loaded raw values per item, keyed by itemKey. Shaping (content
	// vs metadata, YAML, lang detection) is owned by WorkspaceItemDiffViewer.
	type LoadedDiff = {
		state: 'loading' | 'ready' | 'error'
		error?: string
		parentRaw?: unknown
		forkRaw?: unknown
	}
	let loadedDiffs: Record<string, LoadedDiff> = $state({})
	// Per-item summary, derived from the fetched raw value so the tree on
	// the left can show summary above the mono path (matches the picker).
	let summaries: Record<string, string | undefined> = $state({})

	async function loadDiffFor(d: WorkspaceItemDiff) {
		const key = itemKey(d)
		if (loadedDiffs[key]) return
		loadedDiffs[key] = { state: 'loading' }

		try {
			// Source (parent) — empty for items only in fork. Fork — empty for
			// items only in source. We swallow per-side errors so an "added"
			// item still renders cleanly against an empty original.
			const [parentRaw, forkRaw] = await Promise.all([
				d.exists_in_source
					? getItemValue(d.kind, d.path, parentWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined),
				d.exists_in_fork
					? getItemValue(d.kind, d.path, forkWorkspaceId).catch(() => undefined)
					: Promise.resolve(undefined)
			])
			loadedDiffs[key] = { state: 'ready', parentRaw, forkRaw }
			// Prefer the fork's summary (the "current" side); fall back to parent.
			const summary =
				(forkRaw && typeof forkRaw === 'object' && (forkRaw as any).summary) ||
				(parentRaw && typeof parentRaw === 'object' && (parentRaw as any).summary) ||
				undefined
			if (typeof summary === 'string' && summary.trim().length > 0) {
				summaries[key] = summary
			}
		} catch (e) {
			console.error('Fork diff: loadDiff failed', d, e)
			loadedDiffs[key] = {
				state: 'error',
				error: String(e)
			}
		}
	}

	function onDetailsToggle(d: WorkspaceItemDiff, e: Event) {
		const target = e.currentTarget as HTMLDetailsElement | null
		if (target?.open) {
			void loadDiffFor(d)
		}
	}

	function statusBadgeColor(s: DiffStatus): 'green' | 'red' | 'orange' | 'blue' {
		if (s === 'added') return 'green'
		if (s === 'removed') return 'red'
		if (s === 'conflict') return 'orange'
		return 'blue'
	}

	const statusIcons = {
		added: Plus,
		removed: Minus,
		modified: Pencil,
		conflict: AlertTriangle
	}

	// File tree built from the diff paths. Top-level rows mirror
	// WorkspaceItemDrillPicker: `f/foo` and `u/alice` collapse to a single
	// "scope" row, then deeper segments split per `/`. Leaves carry their
	// diff entry.
	type FolderNode = {
		type: 'folder'
		name: string
		fullPath: string
		isScope: boolean
		children: TreeNode[]
	}
	type FileNode = { type: 'file'; name: string; diff: WorkspaceItemDiff }
	type TreeNode = FolderNode | FileNode

	function buildTree(diffs: WorkspaceItemDiff[]): FolderNode {
		const root: FolderNode = {
			type: 'folder',
			name: '',
			fullPath: '',
			isScope: false,
			children: []
		}
		const folderCache = new Map<string, FolderNode>()

		for (const d of diffs) {
			const parts = d.path.split('/')
			if (parts.length < 2) {
				root.children.push({ type: 'file', name: d.path, diff: d })
				continue
			}
			const scopeKey = parts.slice(0, 2).join('/')
			let scope = folderCache.get(scopeKey)
			if (!scope) {
				scope = {
					type: 'folder',
					name: scopeKey,
					fullPath: scopeKey,
					isScope: true,
					children: []
				}
				folderCache.set(scopeKey, scope)
				root.children.push(scope)
			}
			if (parts.length === 2) {
				scope.children.push({ type: 'file', name: scopeKey, diff: d })
				continue
			}
			const rest = parts.slice(2)
			let parent = scope
			let folderKey = scopeKey
			for (let i = 0; i < rest.length - 1; i++) {
				folderKey = `${folderKey}/${rest[i]}`
				let folder = folderCache.get(folderKey)
				if (!folder) {
					folder = {
						type: 'folder',
						name: rest[i],
						fullPath: folderKey,
						isScope: false,
						children: []
					}
					folderCache.set(folderKey, folder)
					parent.children.push(folder)
				}
				parent = folder
			}
			parent.children.push({ type: 'file', name: rest[rest.length - 1], diff: d })
		}

		const sortRec = (n: FolderNode) => {
			n.children.sort((a, b) => {
				if (a.type !== b.type) return a.type === 'folder' ? -1 : 1
				return a.name.localeCompare(b.name)
			})
			for (const c of n.children) if (c.type === 'folder') sortRec(c)
		}
		sortRec(root)
		return root
	}

	// Searchable string per diff: path + summary (when loaded) + kind label.
	// SearchItems' uFuzzy runs fuzzy matching over these. Reads `summaries`
	// directly so the index re-derives as summaries trickle in from
	// loadDiffFor.
	function searchableText(d: WorkspaceItemDiff): string {
		const parts = [d.path, KIND_LABELS[d.kind] ?? d.kind]
		const s = summaries[itemKey(d)]
		if (s) parts.push(s)
		return parts.join(' ')
	}
	let searchedDiffs: (WorkspaceItemDiff & { marked?: string })[] | undefined = $state(undefined)

	// Empty query bypasses SearchItems entirely so we don't wait a tick for
	// the async filter to run after open.
	const filteredDiffs = $derived.by(() => {
		const c = comparison
		if (!c) return [] as WorkspaceItemDiff[]
		const q = searchQuery.trim()
		if (!q) return c.diffs
		return (searchedDiffs ?? []) as WorkspaceItemDiff[]
	})

	const tree = $derived.by(() => {
		const c = comparison
		return c ? buildTree(filteredDiffs) : undefined
	})

	function rowId(d: WorkspaceItemDiff): string {
		return `fork-diff-${itemKey(d)}`
	}

	function scrollToDiff(d: WorkspaceItemDiff) {
		const el = document.getElementById(rowId(d)) as HTMLDetailsElement | null
		if (!el) return
		el.open = true
		el.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	// ── Keyboard nav (matches WorkspaceItemDrillPicker) ─────────────────────
	// Per-folder open/closed state. Defaults to open; user toggles via the
	// <details> summary or via Enter when a folder row is highlighted.
	let folderOpen: Record<string, boolean> = $state({})
	function isFolderOpen(key: string): boolean {
		return folderOpen[key] ?? true
	}
	function folderKey(node: FolderNode): string {
		return `folder:${node.fullPath}`
	}

	type NavEntry =
		| { type: 'folder'; key: string; node: FolderNode }
		| { type: 'file'; key: string; diff: WorkspaceItemDiff }

	function flattenVisible(node: FolderNode): NavEntry[] {
		const out: NavEntry[] = []
		const walk = (n: TreeNode) => {
			if (n.type === 'file') {
				out.push({ type: 'file', key: itemKey(n.diff), diff: n.diff })
				return
			}
			const fkey = folderKey(n)
			out.push({ type: 'folder', key: fkey, node: n })
			if (isFolderOpen(fkey)) for (const c of n.children) walk(c)
		}
		for (const c of node.children) walk(c)
		return out
	}
	const navEntries = $derived(tree ? flattenVisible(tree) : [])
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
		// Same defense as the picker: ignore until the user actually moves the
		// mouse, so a cursor parked over a row doesn't clobber the keyboard
		// highlight when the layout shifts.
		if (mouseActive) highlightedKey = key
	}

	function activateHighlighted() {
		if (!highlightedKey) return
		const entry = entryByKey.get(highlightedKey)
		if (!entry) return
		if (entry.type === 'file') {
			scrollToDiff(entry.diff)
		} else {
			folderOpen[entry.key] = !isFolderOpen(entry.key)
		}
	}

	// The folder containing an entry, as a folder key (or undefined if the
	// entry is at the top scope and has no parent folder). Pure logic lives in
	// forkDiffNav.parentFolderKey (unit-tested).
	function parentFolderKeyFor(entry: NavEntry): string | undefined {
		const path = entry.type === 'folder' ? entry.node.fullPath : entry.diff.path
		return parentFolderKey(entry.type, path)
	}

	function firstChildKey(node: FolderNode): string | undefined {
		const c = node.children[0]
		if (!c) return undefined
		return c.type === 'folder' ? folderKey(c) : itemKey(c.diff)
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
			// On a closed folder: open it. On an open folder: jump to its first
			// child (folder or file). On a file: no-op.
			const entry = highlightedKey ? entryByKey.get(highlightedKey) : undefined
			if (!entry || entry.type !== 'folder') return
			if (!isFolderOpen(entry.key)) {
				e.preventDefault()
				folderOpen[entry.key] = true
				return
			}
			const child = firstChildKey(entry.node)
			if (child) {
				e.preventDefault()
				selectKey(child)
			}
		} else if (e.key === 'ArrowLeft') {
			// On an open folder: collapse it. On a closed folder (or a file):
			// jump to the parent folder.
			const entry = highlightedKey ? entryByKey.get(highlightedKey) : undefined
			if (!entry) return
			if (entry.type === 'folder' && isFolderOpen(entry.key)) {
				e.preventDefault()
				folderOpen[entry.key] = false
				return
			}
			const parent = parentFolderKeyFor(entry)
			if (parent && entryByKey.has(parent)) {
				e.preventDefault()
				selectKey(parent)
			}
		}
	}
</script>

<SearchItems
	filter={searchQuery}
	items={comparison?.diffs ?? []}
	bind:filteredItems={searchedDiffs}
	f={(d: WorkspaceItemDiff) => searchableText(d)}
/>

{#snippet renderTreeNode(node: TreeNode, depth: number)}
	{#if node.type === 'folder'}
		{@const isUserScope = node.isScope && node.name.startsWith('u/')}
		{@const fkey = folderKey(node)}
		{@const open = isFolderOpen(fkey)}
		{@const isHl = fkey === highlightedKey}
		<details
			{open}
			ontoggle={(e) => (folderOpen[fkey] = (e.currentTarget as HTMLDetailsElement).open)}
			class="select-none"
		>
			<summary
				role="option"
				aria-selected={isHl}
				data-nav-key={fkey}
				onmouseenter={() => setHoverHighlight(fkey)}
				class="flex items-center gap-1.5 px-3 py-1 cursor-pointer text-xs font-medium font-mono text-emphasis hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
					? 'bg-surface-hover'
					: ''}"
				style="padding-left: {depth * 12 + 8}px"
			>
				<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
				<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
				{#if isUserScope}
					<User size={12} class="shrink-0 text-tertiary" />
				{:else}
					<Folder size={12} class="shrink-0 text-tertiary" />
				{/if}
				<span class="truncate" title={node.name}>{node.name}</span>
			</summary>
			<div>
				{#each node.children as child}
					{@render renderTreeNode(child, depth + 1)}
				{/each}
			</div>
		</details>
	{:else}
		{@const status = statusOf(node.diff)}
		{@const key = itemKey(node.diff)}
		<WorkspaceItemRow
			kind={node.diff.kind}
			summary={summaries[key]}
			secondary={node.name}
			highlighted={key === highlightedKey}
			navKey={key}
			indent={depth * 12 + 20}
			title={node.diff.path}
			onclick={() => {
				highlightedKey = key
				scrollToDiff(node.diff)
			}}
			onmouseenter={() => setHoverHighlight(key)}
		>
			{#snippet extras()}
				<span
					class="w-1.5 h-1.5 rounded-full shrink-0 {status === 'added'
						? 'bg-green-500'
						: status === 'removed'
							? 'bg-red-500'
							: status === 'conflict'
								? 'bg-orange-500'
								: 'bg-blue-500'}"
				></span>
			{/snippet}
		</WorkspaceItemRow>
	{/if}
{/snippet}

<Drawer bind:this={drawer} size="1200px">
	<DrawerContent
		title="Fork changes"
		on:close={() => drawer?.closeDrawer()}
		documentationLink={undefined}
		noPadding
		overflow_y={false}
	>
		{#snippet titleExtra()}
			<div class="flex items-center gap-2 text-xs text-secondary">
				<GitFork class="w-3.5 h-3.5 shrink-0" />
				<span class="font-medium truncate">{forkWs?.name ?? forkWorkspaceId}</span>
				<ArrowRight class="w-3 h-3 shrink-0 text-tertiary" />
				<span class="font-medium truncate">{parentWs?.name ?? parentWorkspaceId}</span>
				{#if comparison}
					<Badge color="transparent" class="ml-2">
						{comparison.summary.total_diffs} item{comparison.summary.total_diffs !== 1 ? 's' : ''}
					</Badge>
					{#if comparison.summary.conflicts > 0}
						<Badge color="orange">
							<AlertTriangle class="w-3 h-3 inline mr-1" />
							{comparison.summary.conflicts} conflict{comparison.summary.conflicts !== 1 ? 's' : ''}
						</Badge>
					{/if}
				{/if}
			</div>
		{/snippet}
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
			<Button variant="accent" unifiedSize="sm" startIcon={{ icon: GitMerge }} onclick={openReview}>
				Review
			</Button>
		{/snippet}
		<div class="flex flex-row h-full min-h-0">
			{#if comparison && comparison.diffs.length > 0}
				<aside
					bind:this={sidebarRoot}
					onmousemove={() => (mouseActive = true)}
					class="flex-none w-56 border-r border-light flex flex-col min-h-0"
				>
					<div class="px-3 pt-3 pb-2 shrink-0">
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
						{#if tree && tree.children.length > 0}
							{#each tree.children as child}
								{@render renderTreeNode(child, 0)}
							{/each}
						{:else}
							<div class="text-2xs text-tertiary px-3 py-2">No matches</div>
						{/if}
					</div>
				</aside>
			{/if}
			<main class="flex-1 min-w-0 overflow-y-auto">
				<div class="px-3 pt-3 pb-4 flex flex-col gap-3">
					{#if loading && !comparison}
						<div class="flex items-center gap-2 text-sm text-secondary py-8 self-center">
							<Loader2 class="w-4 h-4 animate-spin" />
							Loading comparison...
						</div>
					{:else if error}
						<div class="text-sm text-red-600 dark:text-red-400 py-4">{error}</div>
					{:else if comparison?.skipped_comparison}
						<div class="text-sm text-secondary py-4">
							This fork was created before change tracking was added — diffs are not available.
						</div>
					{:else if comparison && comparison.diffs.length === 0}
						<div class="text-sm text-secondary py-4"
							>No changes between this fork and its parent.</div
						>
					{:else if comparison && filteredDiffs.length === 0}
						<div class="text-sm text-secondary py-4">No files match "{searchQuery}".</div>
					{:else if comparison}
						<div class="flex flex-col gap-2">
							{#each filteredDiffs as d (itemKey(d))}
								{@const key = itemKey(d)}
								{@const status = statusOf(d)}
								{@const StatusIcon = statusIcons[status]}
								{@const loaded = loadedDiffs[key]}
								{@const editUrl = editUrlFor(d)}
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
										<RowIcon kind={d.kind} size={14} />
										<div class="min-w-0 flex-1">
											{#if editUrl}
												<a
													href={editUrl}
													target="_blank"
													rel="noopener noreferrer"
													title={d.path}
													onclick={(e) => e.stopPropagation()}
													class="group inline-flex items-center gap-1 max-w-full text-xs text-primary font-mono truncate hover:underline"
												>
													<span class="truncate">{d.path}</span>
													<ExternalLink
														class="w-3 h-3 shrink-0 opacity-0 group-hover:opacity-60 transition-opacity"
													/>
												</a>
											{:else}
												<div class="text-xs text-primary font-mono truncate" title={d.path}>
													{d.path}
												</div>
											{/if}
										</div>
										<div class="shrink-0 flex items-center gap-2">
											{#if d.ahead > 0}
												<span class="text-2xs text-secondary">{d.ahead} ahead</span>
											{/if}
											{#if d.behind > 0}
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
										{#if !loaded || loaded.state === 'loading'}
											<div class="flex items-center gap-2 text-xs text-secondary p-3">
												<Loader2 class="w-3.5 h-3.5 animate-spin" />
												Loading diff…
											</div>
										{:else if loaded.state === 'error'}
											<div class="text-xs text-red-600 dark:text-red-400">{loaded.error}</div>
										{:else if loaded.state === 'ready'}
											<WorkspaceItemDiffViewer
												kind={d.kind}
												originalRaw={loaded.parentRaw}
												currentRaw={loaded.forkRaw}
												{inlineDiff}
											/>
										{/if}
									</div>
								</details>
							{/each}
						</div>
					{/if}
				</div></main
			></div
		>
	</DrawerContent>
</Drawer>

<style>
	/* Diff rows use a ChevronDown; rotate it back when collapsed. */
	details:not([open]) :global(.chevron) {
		transform: rotate(-90deg);
	}
	/* Tree folder rows: swap chevrons based on the folder's open state. */
	details:not([open]) > .tree-summary :global(.tree-chevron-open) {
		display: none;
	}
	details[open] > .tree-summary :global(.tree-chevron-closed) {
		display: none;
	}
</style>
