<script lang="ts">
	import { buildDiffTree, type AppRootMeta, type TreeNode, type FolderNode } from './diffTree'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import {
		ChevronDown,
		ChevronRight,
		Folder,
		GitMerge,
		Loader2,
		PanelRightClose,
		PanelRightOpen,
		User
	} from 'lucide-svelte'
	import { goto } from '$lib/navigation'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import WorkspaceItemRow from '$lib/components/WorkspaceItemRow.svelte'
	import DraftBadge from '$lib/components/DraftBadge.svelte'
	import { userStore } from '$lib/stores'
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
	import { tick, type Snippet } from 'svelte'
	import ExternalEditLink from '../ExternalEditLink.svelte'
	import {
		actionFor,
		pipelineOf,
		selectableOf,
		type DeployItem,
		type DeployItemState,
		type DeploySegment,
		type PipelineStage
	} from './sessionDeployModel'
	import type { SessionDeployModel, DiffValues } from './sessionDeployModel.svelte'

	// The session Review & Deploy surface. Reads a unified item model (drafts +
	// fork comparison) and renders a folder tree with lifecycle filter segments +
	// selection (left) and a scroll-through diff column with per-block sticky
	// headers (right). S2: everything reads/navigates; the deploy actions and
	// footer buttons render but are inert (wired in S3).
	let {
		model,
		title,
		reviewHref,
		reviewLabel = 'Review',
		editUrlFor = undefined,
		titleExtra
	}: {
		model: SessionDeployModel
		title: string
		reviewHref: string
		reviewLabel?: string
		/** Editor URL for a row (opens in the workspace the item lives in). */
		editUrlFor?: (item: DeployItem) => string | undefined
		titleExtra?: Snippet
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let searchQuery = $state('')
	let diffStyle = $state<'sbs' | 'inline'>('sbs')
	const inlineDiff = $derived(diffStyle === 'inline')
	// Collapse the diff panel to browse the tree at full width; a per-row action
	// (or clicking a row) re-expands it and jumps to the item.
	let rightPanelCollapsed = $state(false)

	export function open() {
		loadedDiffs = {}
		mountedRows = {}
		rightPanelCollapsed = false
		model.load()
		drawer?.openDrawer()
		setTimeout(() => searchInputEl?.focus(), 50)
	}

	// ── State presentation ───────────────────────────────────────────────────
	const STATE_META: Record<
		DeployItemState,
		{ label: string; color: 'blue' | 'green' | 'red' | 'orange' | 'gray' }
	> = {
		draft: { label: 'Draft', color: 'blue' },
		in_fork: { label: 'In fork', color: 'blue' },
		in_parent: { label: 'Done', color: 'green' },
		deleted: { label: 'Deleted', color: 'red' },
		conflict: { label: 'Conflict', color: 'orange' }
	}
	function stateLabel(item: DeployItem): string {
		if (item.state === 'draft' && item.draftOnly) return 'Draft only'
		return STATE_META[item.state].label
	}

	// ── Segments ───────────────────────────────────────────────────────────
	const SEGMENTS: { id: DeploySegment; label: string }[] = [
		{ id: 'to_review', label: 'To review' },
		{ id: 'drafts', label: 'Drafts' },
		{ id: 'in_fork', label: 'In fork' },
		{ id: 'done', label: 'Done' },
		{ id: 'all', label: 'All' }
	]
	// Main (non-fork) sessions have no fork stage — hide the In-fork segment.
	const visibleSegments = $derived(
		model.context.isFork ? SEGMENTS : SEGMENTS.filter((s) => s.id !== 'in_fork')
	)

	// ── Rendered list: segment-filtered, then text-searched ──────────────────
	const segmentItems = $derived(model.visibleItems)

	function searchableText(d: DeployItem): string {
		return [d.displayPath, d.deployKind, d.summary ?? ''].join(' ')
	}
	let searchedItems: (DeployItem & { marked?: string })[] | undefined = $state(undefined)
	const filteredItems = $derived.by(() => {
		const q = searchQuery.trim()
		if (!q) return segmentItems
		return (searchedItems ?? []) as DeployItem[]
	})

	// ── Tree ─────────────────────────────────────────────────────────────────
	const appRootMeta = $derived.by(() => {
		const m = new Map<string, AppRootMeta>()
		for (const it of model.items) {
			if (it.deployKind !== 'raw_app') continue
			m.set(it.displayPath, {
				summaryKey: it.key,
				summary: it.summary,
				hasDraft: it.hasDraft,
				draftOnly: it.draftOnly,
				draftUsers: it.draftUsers,
				draftItemKind: it.draftKind
			})
		}
		return m
	})

	const treeModel = $derived(
		buildDiffTree(
			filteredItems.map((d) => ({ key: d.key, structurePath: d.displayPath, data: d })),
			appRootMeta
		)
	)

	function rowId(d: DeployItem): string {
		return `ws-diff-${d.key}`
	}

	function scrollToDiff(d: DeployItem) {
		const el = document.getElementById(rowId(d)) as HTMLDetailsElement | null
		if (!el) return
		el.open = true
		el.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	async function revealDiff(d: DeployItem, key?: string) {
		if (key) highlightedKey = key
		if (rightPanelCollapsed) {
			rightPanelCollapsed = false
			await tick()
		}
		scrollToDiff(d)
	}

	// ── Selection (folder checkbox toggles its selectable subtree) ────────────
	function collectSelectableKeys(node: TreeNode<DeployItem>): string[] {
		if (node.type === 'file') return selectableOf(node.data) ? [node.key] : []
		const out: string[] = []
		for (const child of node.children) out.push(...collectSelectableKeys(child))
		return out
	}
	function folderChecks(node: FolderNode<DeployItem>): {
		checked: boolean
		indeterminate: boolean
		keys: string[]
	} {
		const keys = collectSelectableKeys(node)
		const sel = keys.filter((k) => model.isSelected(k)).length
		return {
			checked: keys.length > 0 && sel === keys.length,
			indeterminate: sel > 0 && sel < keys.length,
			keys
		}
	}
	// Select-all operates on the visible (segment+search) selectable rows.
	const visibleSelectableKeys = $derived(model.selectableKeysOf(filteredItems))
	const allVisibleSelected = $derived(
		visibleSelectableKeys.length > 0 && visibleSelectableKeys.every((k) => model.isSelected(k))
	)
	const selectedVisibleCount = $derived(
		visibleSelectableKeys.filter((k) => model.isSelected(k)).length
	)

	// ── Keyboard nav ─────────────────────────────────────────────────────────
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
		if (!highlightedKey || !navKeys.includes(highlightedKey)) highlightedKey = navKeys[0]
	})
	function scrollHighlightIntoView() {
		if (!sidebarRoot || !highlightedKey) return
		sidebarRoot
			.querySelector<HTMLElement>(`[data-nav-key="${CSS.escape(highlightedKey)}"]`)
			?.scrollIntoView({ block: 'nearest', behavior: 'smooth' })
	}
	function moveHighlight(delta: 1 | -1) {
		if (navKeys.length === 0) return
		const cur = navKeys.indexOf(highlightedKey ?? '')
		highlightedKey = navKeys[cur < 0 ? 0 : (cur + delta + navKeys.length) % navKeys.length]
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
		if (entry.type === 'file') void revealDiff(entry.data)
		else folderOpen[entry.key] = !isFolderOpen(entry.key)
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

	// ── Diff values (lazy Monaco mount, one resolver via the model) ──────────
	type LoadedDiff = {
		state: 'loading' | 'ready' | 'error'
		error?: string
		before?: unknown
		after?: unknown
	}
	let loadedDiffs: Record<string, LoadedDiff> = $state({})
	let mainScrollEl: HTMLElement | undefined = $state()
	let mountedRows: Record<string, boolean> = $state({})
	const MOUNT_MARGIN = '200px 0px'

	async function loadDiffFor(item: DeployItem) {
		if (loadedDiffs[item.key]) return
		loadedDiffs[item.key] = { state: 'loading' }
		try {
			const { before, after }: DiffValues = await model.loadDiffValues(item)
			loadedDiffs[item.key] = { state: 'ready', before, after }
		} catch (e) {
			console.error('WorkspaceDiffDrawer: loadDiffValues failed', item, e)
			loadedDiffs[item.key] = { state: 'error', error: String(e) }
		}
	}

	function lazyMount(node: HTMLElement, item: DeployItem) {
		const io = new IntersectionObserver(
			(entries) => {
				if (entries.some((e) => e.isIntersecting)) {
					mountedRows[item.key] = true
					void loadDiffFor(item)
					io.disconnect()
				}
			},
			{ root: mainScrollEl ?? null, rootMargin: MOUNT_MARGIN }
		)
		io.observe(node)
		return { destroy: () => io.disconnect() }
	}

	// Raw-app blocks explode into their per-file / per-runnable sub-diffs; other
	// kinds render one before/after viewer.
	function rawAppItems(
		item: DeployItem,
		loaded: LoadedDiff
	): (RawAppFileItem | RawAppRunnableItem)[] {
		return rawAppDiffToItems(
			item.path,
			loaded.before as RawAppish | undefined,
			loaded.after as RawAppish | undefined,
			item.displayPath
		)
	}
</script>

<SearchItems
	filter={searchQuery}
	items={segmentItems}
	bind:filteredItems={searchedItems}
	f={(d: DeployItem) => searchableText(d)}
/>

{#snippet statePill(item: DeployItem)}
	<Badge color={STATE_META[item.state].color} small>{stateLabel(item)}</Badge>
{/snippet}

{#snippet pipeline(stages: PipelineStage[])}
	<div class="flex items-center gap-0.5" aria-hidden="true">
		{#each stages as stage, i}
			{#if i > 0}
				<div class="w-2 h-px bg-gray-300 dark:bg-gray-600"></div>
			{/if}
			<div
				class="w-2 h-2 rounded-full border {stage.status === 'done'
					? 'bg-blue-500 border-blue-500'
					: stage.status === 'current'
						? 'bg-surface border-blue-500'
						: stage.status === 'blocked'
							? 'bg-amber-400 border-amber-500'
							: 'bg-surface border-gray-300 dark:border-gray-600'}"
				title={`${stage.id}: ${stage.status}`}
			></div>
		{/each}
	</div>
{/snippet}

{#snippet renderTreeNode(node: TreeNode<DeployItem>, depth: number)}
	{#if node.type === 'folder'}
		{@const isUserScope = node.isScope && node.name.startsWith('u/')}
		{@const fkey = node.key}
		{@const open = isFolderOpen(fkey)}
		{@const isHl = fkey === highlightedKey}
		{@const checks = folderChecks(node)}
		<details
			{open}
			ontoggle={(e) => (folderOpen[fkey] = (e.currentTarget as HTMLDetailsElement).open)}
			class="select-none"
		>
			{#if node.app}
				{@const appItem = filteredItems.find((it) => it.key === node.app?.summaryKey)}
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
					{#if appItem && selectableOf(appItem)}
						<span onclick={(e) => e.stopPropagation()} class="shrink-0 flex items-center">
							<Checkbox
								checked={model.isSelected(appItem.key)}
								onChange={() => model.toggle(appItem.key)}
							/>
						</span>
					{/if}
					<RowIcon kind="raw_app" size={12} />
					<span
						class="flex-1 min-w-0 truncate text-xs font-normal text-primary {node.app.summary
							? ''
							: 'font-mono'}"
					>
						{node.app.summary ?? node.name}
					</span>
					{#if appItem}{@render statePill(appItem)}{/if}
					{#if node.app.hasDraft}
						<DraftBadge
							is_draft
							draft_only={node.app.draftOnly ?? false}
							draft_users={node.app.draftUsers ?? []}
							itemKind={node.app.draftItemKind}
							currentUsername={$userStore?.username}
						/>
					{/if}
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
					{#if checks.keys.length > 0}
						<span onclick={(e) => e.stopPropagation()} class="shrink-0 flex items-center">
							<Checkbox
								checked={checks.checked}
								onChange={() => model.setSelected(checks.keys, !checks.checked)}
							/>
						</span>
					{/if}
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
		{@const canSelect = selectableOf(d)}
		<div class="relative group/reveal flex items-stretch">
			{#if canSelect}
				<span class="shrink-0 flex items-center pl-3" style="padding-left: {depth * 12 + 8}px">
					<Checkbox checked={model.isSelected(key)} onChange={() => model.toggle(key)} />
				</span>
			{/if}
			<WorkspaceItemRow
				kind={d.deployKind as any}
				iconPath={d.path}
				baseClass="py-1.5"
				singleLine
				summary={d.summary}
				secondary={node.name}
				highlighted={key === highlightedKey}
				navKey={key}
				indent={canSelect ? 0 : depth * 12 - 4}
				title={d.displayPath}
				onclick={() => revealDiff(d, key)}
				onmouseenter={() => setHoverHighlight(key)}
			>
				{#snippet extras()}
					{@render statePill(d)}
					{#if d.hasDraft}
						<DraftBadge
							is_draft
							draft_only={d.draftOnly}
							draft_users={d.draftUsers ?? []}
							itemKind={d.draftKind}
							currentUsername={$userStore?.username}
						/>
					{/if}
					<span class="w-5 shrink-0" aria-hidden="true"></span>
				{/snippet}
			</WorkspaceItemRow>
			<button
				type="button"
				title="Open diff"
				aria-label="Open diff"
				class="absolute right-1.5 top-1/2 -translate-y-1/2 z-10 flex items-center justify-center p-1 rounded text-tertiary opacity-0 group-hover/reveal:opacity-100 focus:opacity-100 hover:text-primary hover:bg-surface-selected transition-opacity"
				onmousedown={(e) => e.preventDefault()}
				onclick={(e) => {
					e.stopPropagation()
					void revealDiff(d, key)
				}}
			>
				<PanelRightOpen size={13} />
			</button>
		</div>
	{/if}
{/snippet}

{#snippet diffBlock(item: DeployItem)}
	{@const loaded = loadedDiffs[item.key]}
	{#if item.state === 'in_parent'}
		<div class="text-2xs text-tertiary p-3">Deployed — no pending changes.</div>
	{:else if !mountedRows[item.key]}
		<div
			use:lazyMount={item}
			class="flex items-center gap-2 text-2xs text-tertiary p-3 min-h-[10rem]"
		>
			<Loader2 class="w-3.5 h-3.5 animate-spin" />
			Diff loads on scroll…
		</div>
	{:else if !loaded || loaded.state === 'loading'}
		<div class="flex items-center gap-2 text-xs text-secondary p-3">
			<Loader2 class="w-3.5 h-3.5 animate-spin" />
			Loading diff…
		</div>
	{:else if loaded.state === 'error'}
		<div class="text-xs text-red-600 dark:text-red-400 p-3">{loaded.error}</div>
	{:else if item.deployKind === 'raw_app'}
		<div class="flex flex-col">
			{#each rawAppItems(item, loaded) as sub (('appPath' in sub ? 'rawapp:' : '') + sub.kind + '/' + sub.path)}
				<div class="border-t border-light first:border-t-0">
					<div class="px-3 py-1.5 text-2xs text-secondary font-mono truncate" title={sub.path}>
						{sub.path}
					</div>
					{#if sub.kind === 'raw_app_file'}
						<WorkspaceItemDiffViewer
							kind="raw_app_file"
							rawFile={sub as RawAppFileItem}
							{inlineDiff}
						/>
					{:else}
						{@const runnable = sub as RawAppRunnableItem}
						<WorkspaceItemDiffViewer
							kind="script"
							originalRaw={runnable.originalRaw}
							currentRaw={runnable.currentRaw}
							{inlineDiff}
						/>
					{/if}
				</div>
			{/each}
		</div>
	{:else}
		<WorkspaceItemDiffViewer
			kind={item.deployKind}
			originalRaw={loaded.before}
			currentRaw={loaded.after}
			{inlineDiff}
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
			{#if model.items.length > 0}
				<Button
					variant="subtle"
					unifiedSize="sm"
					iconOnly
					startIcon={{ icon: rightPanelCollapsed ? PanelRightOpen : PanelRightClose }}
					title={rightPanelCollapsed ? 'Show diff panel' : 'Hide diff panel'}
					onclick={() => (rightPanelCollapsed = !rightPanelCollapsed)}
				/>
			{/if}
			<Button
				variant="accent"
				unifiedSize="sm"
				startIcon={{ icon: GitMerge }}
				onclick={() => goto(reviewHref)}
			>
				{reviewLabel}
			</Button>
		{/snippet}
		<div class="flex flex-col h-full min-h-0">
			<div class="flex flex-row flex-1 min-h-0">
				{#if model.items.length > 0}
					<aside
						bind:this={sidebarRoot}
						onmousemove={() => (mouseActive = true)}
						class="{rightPanelCollapsed
							? 'flex-1'
							: 'flex-none w-72 border-r border-light'} flex flex-col min-h-0"
					>
						<!-- Filter segments -->
						<div class="px-2 pt-2 shrink-0 flex flex-wrap gap-1">
							{#each visibleSegments as seg}
								<button
									type="button"
									class="text-2xs px-2 py-1 rounded border transition-colors {model.segment ===
									seg.id
										? 'border-accent bg-surface-selected text-primary'
										: 'border-light text-secondary hover:bg-surface-hover'}"
									onclick={() => model.setSegment(seg.id)}
								>
									{seg.label} · {model.counts[seg.id]}
								</button>
							{/each}
						</div>
						<div class="px-3 pt-2 pb-1 shrink-0">
							<input
								bind:this={searchInputEl}
								type="search"
								bind:value={searchQuery}
								placeholder="Filter files..."
								onkeydown={handleSearchKeydown}
								class="w-full text-xs px-2 py-1 rounded border border-light bg-surface focus:outline-none focus:border-accent"
							/>
						</div>
						<!-- Selection bar -->
						{#if visibleSelectableKeys.length > 0}
							<div
								class="px-3 pb-1 shrink-0 flex items-center justify-between text-2xs text-secondary"
							>
								<span>{selectedVisibleCount} of {visibleSelectableKeys.length} selected</span>
								<button
									type="button"
									class="text-accent hover:underline"
									onclick={() => model.setSelected(visibleSelectableKeys, !allVisibleSelected)}
								>
									{allVisibleSelected ? 'Clear' : 'Select all'}
								</button>
							</div>
						{/if}
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
				{#if !rightPanelCollapsed}
					<main bind:this={mainScrollEl} class="flex-1 min-w-0 overflow-y-auto">
						<div class="px-3 pt-3 pb-4 flex flex-col gap-3">
							{#if model.loading && model.items.length === 0}
								<div class="flex items-center gap-2 text-sm text-secondary py-8 self-center">
									<Loader2 class="w-4 h-4 animate-spin" />
									Loading comparison...
								</div>
							{:else if model.error}
								<div class="text-sm text-red-600 dark:text-red-400 py-4">{model.error}</div>
							{:else if model.notice}
								<div class="text-sm text-secondary py-4">{model.notice}</div>
							{:else if model.items.length === 0}
								<div class="text-sm text-secondary py-4">No changes.</div>
							{:else if filteredItems.length === 0}
								<div class="text-sm text-secondary py-4">
									{searchQuery.trim() ? 'No files match.' : 'Nothing in this filter.'}
								</div>
							{:else}
								<div class="flex flex-col gap-2">
									{#each filteredItems as d (d.key)}
										{@const action = actionFor(d, model.context)}
										{@const editUrl = editUrlFor?.(d)}
										<details
											open
											id={rowId(d)}
											class="border border-light rounded-md bg-surface scroll-mt-2"
										>
											<summary
												class="sticky top-0 z-30 bg-surface flex items-center gap-2 px-3 py-2 cursor-pointer list-none [&::-webkit-details-marker]:hidden border-b border-transparent rounded-md"
											>
												<ChevronDown
													class="w-3.5 h-3.5 shrink-0 text-tertiary transition-transform chevron"
												/>
												<RowIcon kind={d.deployKind as any} path={d.path} size={14} />
												<div class="min-w-0 flex-1">
													{#if editUrl}
														<ExternalEditLink
															href={editUrl}
															title={d.displayPath}
															class="text-xs text-primary font-mono truncate"
														>
															<span class="truncate">{d.displayPath}</span>
														</ExternalEditLink>
													{:else}
														<div
															class="text-xs text-primary font-mono truncate"
															title={d.displayPath}
														>
															{d.displayPath}
														</div>
													{/if}
												</div>
												<div class="shrink-0 flex items-center gap-2">
													{@render pipeline(pipelineOf(d, model.context))}
													{@render statePill(d)}
													{#if action.op !== 'none'}
														<!-- Deploy action is inert in S2; wired in S3. -->
														<Button
															variant="accent"
															unifiedSize="xs"
															disabled
															title="Deploy wiring lands in a later stage"
														>
															{action.label}
														</Button>
													{/if}
												</div>
											</summary>
											<div
												class="border-t border-light bg-surface-tertiary rounded-b-md overflow-hidden"
											>
												{@render diffBlock(d)}
											</div>
										</details>
									{/each}
								</div>
							{/if}
						</div>
					</main>
				{/if}
			</div>
			<!-- Footer: selection summary + (inert) deploy buttons -->
			{#if model.items.length > 0}
				{@const parentTarget = model.context.parentName ?? 'parent'}
				<div
					class="shrink-0 border-t border-light bg-surface px-3 py-2 flex items-center justify-between gap-2 text-xs"
				>
					<div class="text-secondary min-w-0 truncate">
						{#if model.context.isFork}
							{model.footer.toFork} to fork · {model.footer.toParent} to {parentTarget} selected{#if model.footer.conflicts > 0}
								· {model.footer.conflicts} blocked{/if}
						{:else}
							{model.footer.toParent} selected to deploy
						{/if}
					</div>
					<div class="flex items-center gap-1.5 shrink-0">
						{#if model.context.isFork}
							<Button variant="default" unifiedSize="xs" disabled title="Wired in a later stage">
								Deploy {model.footer.toFork} to fork
							</Button>
							<Button variant="accent" unifiedSize="xs" disabled title="Wired in a later stage">
								Deploy {model.footer.toParent} to {parentTarget}
							</Button>
						{:else}
							<Button variant="accent" unifiedSize="xs" disabled title="Wired in a later stage">
								Deploy {model.footer.toParent}
							</Button>
						{/if}
					</div>
				</div>
			{/if}
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
