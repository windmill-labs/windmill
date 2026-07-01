<script lang="ts">
	import { buildDiffTree, type AppRootMeta, type TreeNode } from './diffTree'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import {
		AlertTriangle,
		ChevronDown,
		ChevronRight,
		ExternalLink,
		Folder,
		Loader2,
		User
	} from 'lucide-svelte'
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
	import type { Snippet } from 'svelte'
	import ExternalEditLink from '../ExternalEditLink.svelte'
	import OnBehalfOfSelector from '../OnBehalfOfSelector.svelte'
	import ParentWorkspaceProtectionAlert from '../ParentWorkspaceProtectionAlert.svelte'
	import {
		actionFor,
		badgeOf,
		changeOpOf,
		isOnBehalfEligible,
		pipelineOf,
		type BadgeKind,
		type DeployItem,
		type PipelineStage
	} from './sessionDeployModel'
	import type { SessionDeployModel, DiffValues } from './sessionDeployModel.svelte'

	// The session "Edits" surface. Reads a unified item model (drafts + fork
	// comparison, scoped to what this session edited) and renders a folder tree
	// (left) plus a scroll-through diff column with per-block sticky headers
	// (right). Deploy is granular per-item; batch/PR flows are handed to the
	// compare page via the drift banner + footer links.
	let {
		model,
		title,
		editUrlFor = undefined,
		titleExtra,
		compareSessionHref
	}: {
		model: SessionDeployModel
		title: string
		/** Editor URL for a row (opens in the workspace the item lives in). */
		editUrlFor?: (item: DeployItem) => string | undefined
		titleExtra?: Snippet
		/** Compare page for this session's edits (footer) — preselected via
		 *  `from_session`. Where "deploy all / request review" happens. */
		compareSessionHref?: string
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let searchQuery = $state('')
	let diffStyle = $state<'sbs' | 'inline'>('sbs')
	const inlineDiff = $derived(diffStyle === 'inline')
	// Parent-workspace protection: when direct deploy is disallowed, the per-row
	// parent deploy is blocked and the user routes through the compare page.
	let canDeployToParent = $state(true)

	export function open() {
		loadedDiffs = {}
		mountedRows = {}
		model.load()
		drawer?.openDrawer()
		setTimeout(() => searchInputEl?.focus(), 50)
	}

	// ── Badge presentation ─────────────────────────────────────────────────────
	// The single parent-comparison badge, by priority (conflict > draft > ahead).
	// `draft` renders as a plain indigo pill (New draft / Draft); `none` renders nothing.
	// Colors: ahead=green, conflict=red, deployed=purple.
	const BADGE_META: Record<
		Exclude<BadgeKind, 'draft' | 'none'>,
		{ label: string; color: 'green' | 'red' | 'violet' }
	> = {
		ahead: { label: 'Ahead', color: 'green' },
		conflict: { label: 'Conflict', color: 'red' },
		deployed: { label: 'Deployed', color: 'violet' }
	}
	// One footprint for every status pill (draft / ahead / conflict / deployed) so
	// they line up at a compact, uniform size.
	const STATUS_BADGE_CLASS = 'px-1.5 py-0.5 gap-0.5 text-2xs'
	// Change-operation for the type-icon overlay: only add/delete get a mark; a
	// plain modification shows the bare icon.
	function iconOpOf(item: DeployItem): 'add' | 'delete' | undefined {
		const op = changeOpOf(item)
		return op === 'modify' ? undefined : op
	}
	// Active-dot accent for the pipeline indicator, colored by the row's badge.
	function dotAccent(badge: BadgeKind): { border: string; bg: string } {
		switch (badge) {
			case 'conflict':
				return { border: 'border-red-500', bg: 'bg-red-500' }
			case 'ahead':
				return { border: 'border-green-500', bg: 'bg-green-500' }
			case 'draft':
				return { border: 'border-indigo-500', bg: 'bg-indigo-500' }
			default:
				return { border: 'border-blue-500', bg: 'bg-blue-500' }
		}
	}

	// ── Rendered list: all session items, then text-searched (no filter tabs) ──
	const segmentItems = $derived(model.items)

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
		const el = document.getElementById(rowId(d))
		if (!el) return
		el.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	function revealDiff(d: DeployItem, key?: string) {
		if (key) highlightedKey = key
		scrollToDiff(d)
	}

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

{#snippet badgePill(item: DeployItem, badge: Exclude<BadgeKind, 'draft' | 'none'>)}
	{@const parentTarget = model.context.parentName ?? 'the parent'}
	{@const hint =
		badge === 'ahead'
			? `Deployed in this fork but not yet in ${parentTarget} — deploy it to promote the change`
			: badge === 'deployed'
				? `Deployed in ${parentTarget}`
				: undefined}
	<Badge color={BADGE_META[badge].color} small class={STATUS_BADGE_CLASS} title={hint}>
		{BADGE_META[badge].label}
	</Badge>
{/snippet}

<!-- The single row badge, derived by priority. Conflict carries a ⚠ marker;
     clean+sync is bare. Everything here was edited by the AI this session, so the
     draft badge is a plain pill — no author avatars. -->
{#snippet rowBadge(item: DeployItem)}
	{@const badge = badgeOf(item)}
	{#if badge === 'draft'}
		<Badge
			color="indigo"
			small
			class={STATUS_BADGE_CLASS}
			title={item.draftOnly ? 'Never deployed and is only a draft' : 'Is deployed and has a draft'}
		>
			{item.draftOnly ? 'Draft only' : 'Draft'}
		</Badge>
	{:else if badge === 'conflict'}
		<Badge color="red" small class={STATUS_BADGE_CLASS}>
			<AlertTriangle class="h-3 w-3 shrink-0" />
			{BADGE_META.conflict.label}
		</Badge>
	{:else if badge !== 'none'}
		{@render badgePill(item, badge)}
	{/if}
{/snippet}

{#snippet pipeline(stages: PipelineStage[], accent: { border: string; bg: string })}
	<div class="flex items-center gap-0.5" aria-hidden="true">
		{#each stages as stage, i}
			{#if i > 0}
				<div class="w-2 h-px bg-gray-300 dark:bg-gray-600"></div>
			{/if}
			<div
				class="w-2 h-2 rounded-full border {stage.status === 'done'
					? 'bg-blue-500 border-blue-500'
					: stage.status === 'current'
						? 'bg-surface ' + accent.border
						: stage.status === 'blocked'
							? accent.bg + ' ' + accent.border
							: 'bg-surface border-gray-300 dark:border-gray-600'}"
				title={`${stage.id}: ${stage.status}`}
			></div>
		{/each}
	</div>
{/snippet}

<!-- Transient busy states only. A successful deploy's "Deployed" confirmation is
     rendered by the detail header as a FALLBACK (after the next action) so a
     non-terminal deploy (draft → fork) advances to its next step instead of
     freezing on the chip. -->
{#snippet deployBusy(item: DeployItem)}
	{@const s = model.statusOf(item.key)}
	{#if s?.status === 'loading'}
		<span class="inline-flex items-center gap-1 text-2xs text-secondary">
			<Loader2 class="w-3 h-3 animate-spin" />Deploying…
		</span>
	{:else if s?.status === 'failed'}
		<span class="inline-flex items-center gap-1" title={s.error}>
			<Badge color="red" small>Failed</Badge>
			<Button
				variant="subtle"
				unifiedSize="xs"
				disabled={model.deploying}
				onclick={() => model.deployRow(item)}
			>
				Retry
			</Button>
		</span>
	{/if}
{/snippet}

{#snippet renderTreeNode(node: TreeNode<DeployItem>, depth: number)}
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
				{@const appItem = filteredItems.find((it) => it.key === node.app?.summaryKey)}
				<summary
					role="option"
					aria-selected={isHl}
					data-nav-key={fkey}
					onmouseenter={() => setHoverHighlight(fkey)}
					title={node.fullPath}
					class="flex items-center gap-2 pl-3 pr-1 py-2 rounded-md cursor-pointer hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
						? 'bg-surface-hover'
						: ''}"
					style="padding-left: {depth * 12 + 8}px"
				>
					<RowIcon kind="raw_app" size={12} op={appItem ? iconOpOf(appItem) : undefined} />
					<span
						class="flex-1 min-w-0 truncate text-xs font-normal text-primary {node.app.summary
							? ''
							: 'font-mono'}"
					>
						{node.app.summary ?? node.name}
					</span>
					{#if appItem}
						{@render rowBadge(appItem)}
					{/if}
					<span class="w-5 flex items-center justify-center shrink-0">
						<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
						<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
					</span>
				</summary>
			{:else}
				<summary
					role="option"
					aria-selected={isHl}
					data-nav-key={fkey}
					onmouseenter={() => setHoverHighlight(fkey)}
					class="flex items-center gap-2 pl-3 pr-1 py-2 rounded-md cursor-pointer text-xs font-normal font-mono text-secondary hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
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
					<span class="w-5 flex items-center justify-center shrink-0">
						<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
						<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
					</span>
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
		<!-- Indentation lives on the wrapper (outside the row's hover surface) so the
		     highlight starts at the row content instead of bleeding across the indent. -->
		<div class="flex items-stretch" style="padding-left: {depth * 12 + 4}px">
			<WorkspaceItemRow
				kind={d.deployKind as any}
				iconPath={d.path}
				baseClass="py-2 min-w-0 pr-1 pl-1 rounded-md"
				singleLine
				summary={d.summary}
				secondary={node.name}
				iconOp={iconOpOf(d)}
				highlighted={key === highlightedKey}
				navKey={key}
				indent={0}
				title={d.displayPath}
				onclick={() => revealDiff(d, key)}
				onmouseenter={() => setHoverHighlight(key)}
			>
				{#snippet extras()}
					{@render rowBadge(d)}
				{/snippet}
			</WorkspaceItemRow>
		</div>
	{/if}
{/snippet}

{#snippet diffBlock(item: DeployItem)}
	{@const loaded = loadedDiffs[item.key]}
	{#if item.done}
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
		{/snippet}
		<div class="flex flex-col h-full min-h-0">
			{#if model.context.isFork && model.context.parentWorkspaceId}
				<!-- Horizontal padding only: collapses to zero height (no gap) when the
				     alert renders nothing, which is the case unless a rule is active. -->
				<div class="shrink-0 px-3">
					<ParentWorkspaceProtectionAlert
						parentWorkspaceId={model.context.parentWorkspaceId}
						onUpdateCanDeploy={(c) => (canDeployToParent = c)}
					/>
				</div>
			{/if}
			<div class="flex flex-row flex-1 min-h-0">
				{#if model.items.length > 0}
					<aside
						bind:this={sidebarRoot}
						onmousemove={() => (mouseActive = true)}
						class="flex-none w-96 border-r border-light flex flex-col min-h-0 pt-3 pl-2 pr-2"
					>
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
						<div
							class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden pl-2 pr-1 pt-2 pb-3 flex flex-col gap-1"
						>
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
					<div class="px-3 pt-0 pb-4 flex flex-col gap-3">
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
							<div class="text-sm text-secondary py-4">No files match.</div>
						{:else}
							<div class="-mx-3 flex flex-col divide-y divide-light border-y border-light">
								{#each filteredItems as d (d.key)}
									{@const action = actionFor(d, model.context)}
									{@const editUrl = editUrlFor?.(d)}
									{@const status = model.statusOf(d.key)}
									<div id={rowId(d)} class="bg-surface scroll-mt-2">
										<div
											class="sticky top-0 z-30 bg-surface flex items-center gap-2 px-3 py-2 border-b border-transparent"
										>
											<RowIcon
												kind={d.deployKind as any}
												path={d.path}
												size={14}
												op={iconOpOf(d)}
											/>
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
												{@render pipeline(pipelineOf(d, model.context), dotAccent(badgeOf(d)))}
												{@render rowBadge(d)}
												{#if status?.status === 'loading' || status?.status === 'failed'}
													{@render deployBusy(d)}
												{:else if action.op !== 'none'}
													{@const needsOb =
														action.op === 'deploy_item' && model.needsOnBehalfChoice(d)}
													{@const parentBlocked =
														action.targetStage === 'parent' && !canDeployToParent}
													<!-- Deploy target: draft deploys in place (current ws); everything
													     else writes into the parent. Preflight the user's permission there. -->
													{@const deployTarget =
														action.op === 'deploy_draft'
															? model.context.currentWorkspaceId
															: model.context.parentWorkspaceId}
													{@const perm = model.deployPermission(deployTarget)}
													{#if action.op === 'deploy_item' && isOnBehalfEligible(d.deployKind) && model.context.parentWorkspaceId}
														<OnBehalfOfSelector
															targetWorkspace={model.context.parentWorkspaceId}
															targetValue={model.targetOnBehalf(d)}
															selected={model.onBehalfChoiceOf(d.key)}
															onSelect={(choice, details) =>
																model.setOnBehalfChoice(d.key, choice, details)}
															kind={d.deployKind}
															canPreserve={model.canPreserveOnBehalf}
														/>
													{/if}
													{#if action.secondary?.length}
														<Button
															variant="subtle"
															unifiedSize="sm"
															destructive
															disabled={model.deploying}
															onclick={() => model.discardRow(d)}
														>
															{action.secondary[0].label}
														</Button>
													{/if}
													<Button
														variant="accent"
														unifiedSize="sm"
														disabled={model.deploying || needsOb || parentBlocked || !perm.ok}
														title={!perm.ok
															? perm.reason
															: needsOb
																? 'Choose "run on behalf of" first'
																: parentBlocked
																	? 'Direct deploy to the parent is disabled — deploy from the compare page'
																	: undefined}
														onclick={() => model.deployRow(d)}
													>
														{action.label}
													</Button>
												{:else if status?.status === 'deployed'}
													<!-- Deploy succeeded and there's no further step yet (the fork
													     comparison catches up async — once it does and a next step
													     exists, the action button above replaces this). -->
													<Badge color="green" small>Deployed</Badge>
												{:else if d.parent === 'conflict'}
													<!-- No safe one-click resolve: both sides changed. Reconcile by
													     hand in the editor. -->
													<span
														class="text-2xs text-tertiary italic"
														title="Both this fork and {model.context.parentName ??
															'the parent'} changed this item — open it in the editor and reconcile manually"
													>
														Resolve manually
													</span>
												{/if}
											</div>
										</div>
										<div class="bg-surface-tertiary overflow-hidden">
											{@render diffBlock(d)}
										</div>
									</div>
								{/each}
							</div>
						{/if}
					</div>
				</main>
			</div>
			<!-- Batch / PR (deploy all, request review, reconcile) is the compare page's
			     job — the dock deploys per item; this footer is just the doorway there. -->
			{#if model.items.length > 0 && compareSessionHref}
				<div
					class="shrink-0 border-t border-light bg-surface px-3 py-1.5 flex items-center justify-end text-xs"
				>
					<a
						href={compareSessionHref}
						target="_blank"
						rel="noopener noreferrer"
						class="inline-flex items-center gap-1 font-medium text-accent hover:underline shrink-0"
					>
						Deploy all / request review on compare page
						<ExternalLink class="w-3 h-3" />
					</a>
				</div>
			{/if}
		</div>
	</DrawerContent>
</Drawer>

<style>
	details:not([open]) > .tree-summary :global(.tree-chevron-open) {
		display: none;
	}
	details[open] > .tree-summary :global(.tree-chevron-closed) {
		display: none;
	}
</style>
