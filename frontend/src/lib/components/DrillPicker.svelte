<!--
@component
Generic drill-through picker. Renders a tree of branches and leaves; one
level is shown at a time. The host supplies the tree shape — workspace
items, chat context elements, etc. all map to the same component.

  - **Root** (no scope): the tree's top-level entries.
  - **Branch** (scope = `[...keys]`): the children of the branch resolved
    by walking the tree along the scope chain.

Clicking a row drills *down*; the chevron-left in the header walks one
level *up*. Filter (internal or `externalFilter`) is global across all
leaves and ignores the current scope.
-->
<script lang="ts" generics="L">
	import { ChevronLeft, ChevronRight, Loader2 } from 'lucide-svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import { generateRandomString } from '$lib/utils'
	import { onMount, untrack, type Snippet } from 'svelte'
	import {
		collectLeavesGrouped,
		leafHaystack,
		resolveScope,
		scopeChain,
		type DrillBranch,
		type DrillLeaf,
		type DrillNode
	} from './drillPicker'

	interface Props {
		tree: DrillNode<L>[]
		onPick: (leaf: DrillLeaf<L>) => void
		/** Drill path to land on initially. Empty = root. */
		initialScope?: string[]
		/** Composite key of the row to highlight initially. */
		initialHighlight?: string
		/** When set (any string, incl. ''), the host owns search input.
		 * The internal search field is hidden and the host is expected to
		 * forward keydown events via `handleKeydown`. */
		externalFilter?: string
		autoFocus?: boolean
		/** Drop the outer fixed width / max height wrapper. */
		flush?: boolean
		/** Custom renderer for the icon column of a leaf. */
		leafIcon?: Snippet<[DrillLeaf<L>]>
		/** Custom renderer for the icon column of a branch (used inside
		 * the entry row AND as the leading icon in the breadcrumb header). */
		branchIcon?: Snippet<[DrillBranch<L>]>
		/** Override for a leaf row's secondary text in the **drilled view**
		 * (search results always show `leaf.secondary` to keep absolute
		 * paths visible globally). Returns `undefined` to defer to
		 * `leaf.secondary`. Used by the workspace adapter to render
		 * scope-relative paths once the user has drilled into a folder. */
		leafSecondary?: (leaf: DrillLeaf<L>, scope: string[]) => string | undefined
		/** Fires whenever the scope changes. Lets the host trigger lazy
		 * data loading for the branch the user drilled into. */
		onScopeChange?: (scope: string[]) => void
		/** Fires whenever the EFFECTIVE filter changes (internal OR external).
		 * Lets the host trigger global preloads when the user starts searching
		 * — needed in internal-filter mode where the host can't observe the
		 * picker's own search box otherwise. */
		onFilterChange?: (filter: string) => void
	}

	let {
		tree,
		onPick,
		initialScope,
		initialHighlight,
		externalFilter,
		autoFocus = true,
		flush = false,
		leafIcon,
		branchIcon,
		leafSecondary,
		onScopeChange,
		onFilterChange
	}: Props = $props()

	let searchInput: TextInput | undefined = $state()
	let pickerRoot: HTMLElement | undefined = $state()
	const instanceId = generateRandomString(8)
	const listboxId = `dpkr-list-${instanceId}`
	const idFor = (key: string) => `dpkr-${instanceId}-${key.replace(/[^a-zA-Z0-9-]/g, '_')}`

	export function focus() {
		searchInput?.focus()
	}

	// Sibling-popover open: melt-ui's `openFocus` runs once during the
	// close→open transition; the picker may not be mounted yet. Retry
	// after settle. Skipped when `autoFocus` is false (host keeps focus)
	// or when the search input is not rendered (external filter mode).
	onMount(() => {
		if (!autoFocus || externalFilter !== undefined) return
		const t = setTimeout(focus, 50)
		return () => clearTimeout(t)
	})

	let scope = $state<string[]>(untrack(() => initialScope ?? []))
	let internalFilter = $state('')
	const filter = $derived(externalFilter ?? internalFilter)
	const isSearching = $derived(filter.trim() !== '')

	$effect(() => {
		void scope
		onScopeChange?.(scope)
	})

	$effect(() => {
		void filter
		onFilterChange?.(filter)
	})

	/** Tracks whether the last user action was mouse movement (true) or
	 * keyboard nav (false). When false, row `mouseenter` events are
	 * ignored — prevents the cursor from stealing the keyboard-driven
	 * highlight as rows shift under it during scope changes. Re-enabled
	 * on `mousemove`. Starts `false` so the synthetic `mouseenter` fired
	 * when the popover mounts under a stationary cursor doesn't clobber
	 * `initialHighlight`. */
	let mouseActive = $state(false)

	const currentBranch = $derived(resolveScope(tree, scope))
	const entries = $derived<DrillNode<L>[]>(
		scope.length === 0 ? tree : (currentBranch?.children ?? [])
	)

	// Flat leaf pool for global search. Skips branches flagged
	// `omitFromSearch` (e.g. workspace cross-kind 'all' branch).
	const searchPool = $derived(collectLeavesGrouped(tree))
	type SearchEntry = { leaf: DrillLeaf<L>; group: DrillBranch<L> | null; _key: string }
	const searchItems = $derived<SearchEntry[]>(
		searchPool.map(({ leaf, group }) => ({ leaf, group, _key: leaf.key }))
	)
	let searchedItems: (SearchEntry & { marked: string })[] | undefined = $state(undefined)

	// Group filtered results by their nearest-branch ancestor for display.
	const searchResultsByGroup = $derived.by(() => {
		const groups = new Map<
			string,
			{ group: DrillBranch<L> | null; items: (SearchEntry & { marked: string })[] }
		>()
		if (!searchedItems) return [] as { group: DrillBranch<L> | null; items: SearchEntry[] }[]
		for (const r of searchedItems) {
			const gkey = r.group?.key ?? '__none'
			const existing = groups.get(gkey)
			if (existing) existing.items.push(r)
			else groups.set(gkey, { group: r.group, items: [r] })
		}
		return Array.from(groups.values())
	})

	type Entry =
		| { type: 'branch'; key: string; node: DrillBranch<L> }
		| { type: 'leaf'; key: string; node: DrillLeaf<L> }

	const entryList = $derived<Entry[]>(
		entries.map((n) =>
			n.type === 'branch'
				? { type: 'branch' as const, key: n.key, node: n }
				: { type: 'leaf' as const, key: n.key, node: n }
		)
	)

	const navKeys = $derived(
		isSearching
			? (searchedItems ?? ([] as typeof searchItems)).map((r) => r.leaf.key)
			: entryList.map((e) => e.key)
	)

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
		// Ignored until the user actually moves the mouse. Prevents the
		// cursor (parked over a row) from clobbering keyboard-driven
		// selection when the layout shifts beneath it.
		if (mouseActive) highlightedKey = key
	}

	function pick(leaf: DrillLeaf<L>) {
		if (leaf.current || leaf.disabled) return
		onPick(leaf)
	}

	function activate(key: string | undefined) {
		if (!key) return
		if (isSearching) {
			const found = (searchedItems ?? []).find((r) => r.leaf.key === key)
			if (found) pick(found.leaf)
			return
		}
		const entry = entryList.find((e) => e.key === key)
		if (!entry) return
		drill(entry)
	}

	function drill(entry: Entry) {
		if (entry.type === 'branch') {
			scope = [...scope, entry.key]
		} else {
			pick(entry.node)
		}
	}

	function goUp() {
		if (scope.length === 0) return
		const leaving = scope[scope.length - 1]
		scope = scope.slice(0, -1)
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
		} else if (e.key === 'Enter' || e.key === 'Tab') {
			// Tab mirrors Enter so the inline `@<word>` mention completes
			// without losing focus to the next form control (matches the
			// previous `AvailableContextList` behavior). Guard on a
			// highlighted row so an unrelated Tab in an empty picker still
			// falls through to natural focus movement.
			if (e.key === 'Tab' && !highlightedKey) return
			e.preventDefault()
			e.stopPropagation()
			mouseActive = false
			activate(highlightedKey)
		} else if (
			(e.key === 'ArrowLeft' || e.key === 'Backspace') &&
			filter === '' &&
			scope.length > 0
		) {
			// Walk up the tree. Only when search is empty — otherwise these
			// keys would hijack cursor movement / character deletion.
			e.preventDefault()
			e.stopPropagation()
			mouseActive = false
			goUp()
		} else if (e.key === 'ArrowRight' && filter === '' && !isSearching) {
			// Drill into the highlighted branch. Leaves are reserved for
			// Enter (more deliberate, since picking navigates away).
			const entry = entryList.find((en) => en.key === highlightedKey)
			if (entry && entry.type === 'branch') {
				e.preventDefault()
				e.stopPropagation()
				mouseActive = false
				drill(entry)
			}
		}
	}

	export function handleKeydown(e: KeyboardEvent) {
		handleSearchKeydown(e)
	}

	export function pickHighlighted() {
		activate(highlightedKey)
	}

	// Breadcrumb header — labels of branches along the scope chain.
	const headerChain = $derived(scopeChain(tree, scope))
	const headerSegments = $derived(headerChain.map((b) => b.label))
	const headerLabel = $derived(headerSegments.join(' › '))

	/** Number of intermediate segments to hide behind a `…`. Always keep
	 * the first segment (kind / category) and the deepest. Bumped up by
	 * the measurement effect below. */
	let hiddenCount = $state(0)

	const headerLabelDisplay = $derived.by(() => {
		if (headerSegments.length <= 2 || hiddenCount === 0) return headerLabel
		return [headerSegments[0], '…', ...headerSegments.slice(1 + hiddenCount)].join(' › ')
	})

	let breadcrumbSpan: HTMLElement | undefined = $state()
	let lastSegmentsKey = ''

	/** Measurement loop: each pass reads `scrollWidth > clientWidth` on
	 * the truncated span; if overflowing and there's still an intermediate
	 * segment to drop, increment `hiddenCount`. Mutating `hiddenCount`
	 * re-renders and re-fires this effect, so the loop self-terminates
	 * either when the text fits or when only [first, …, leaf] remain
	 * (`truncate-start` then polishes any final overflow). When the
	 * breadcrumb itself changes (new scope), reset to 0 first. */
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
		void hiddenCount
		if (!breadcrumbSpan) return
		const maxHide = Math.max(0, headerSegments.length - 2)
		if (hiddenCount >= maxHide) return
		queueMicrotask(() => {
			if (!breadcrumbSpan) return
			if (breadcrumbSpan.scrollWidth > breadcrumbSpan.clientWidth + 1) {
				hiddenCount = hiddenCount + 1
			}
		})
	})

	const branchLoading = $derived(currentBranch?.loading ?? false)
</script>

<SearchItems
	{filter}
	items={isSearching ? searchItems : []}
	bind:filteredItems={searchedItems}
	f={(x: SearchEntry) => leafHaystack(x.leaf)}
	opts={{}}
/>

{#snippet defaultLeafIcon(leaf: DrillLeaf<L>)}
	{#if leafIcon}
		{@render leafIcon(leaf)}
	{:else if leaf.icon}
		{@const Icon = leaf.icon}
		<Icon size={12} class="shrink-0" />
	{/if}
{/snippet}

{#snippet defaultBranchIcon(branch: DrillBranch<L>)}
	{#if branchIcon}
		{@render branchIcon(branch)}
	{:else if branch.icon}
		{@const Icon = branch.icon}
		<Icon size={12} class="shrink-0 text-tertiary" />
	{/if}
{/snippet}

{#snippet leafRow(leaf: DrillLeaf<L>, secondary: string | undefined, baseClass: string)}
	{@const key = leaf.key}
	{@const isHl = key === highlightedKey}
	{@const isCur = !!leaf.current}
	<button
		type="button"
		id={idFor(key)}
		role="option"
		aria-selected={isHl}
		data-nav-key={key}
		aria-current={isCur ? 'true' : undefined}
		class="w-full text-left flex items-center gap-2 px-3 transition-colors {baseClass} {isHl
			? 'bg-surface-hover'
			: ''} {isCur ? 'cursor-default text-emphasis font-medium' : ''} {leaf.disabled
			? 'opacity-50 cursor-not-allowed'
			: ''}"
		disabled={leaf.disabled}
		onmousedown={(e) => e.preventDefault()}
		onclick={() => pick(leaf)}
		onmouseenter={() => setHoverHighlight(key)}
	>
		{@render defaultLeafIcon(leaf)}
		<div class="min-w-0 flex-1">
			{#if leaf.secondary}
				<div class="text-xs text-primary font-normal truncate">{leaf.label}</div>
				<div class="text-2xs text-hint font-normal font-mono truncate">
					{secondary ?? leaf.secondary}
				</div>
			{:else}
				<div class="text-xs text-primary font-normal font-mono truncate">
					{secondary ?? leaf.label}
				</div>
			{/if}
		</div>
	</button>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={pickerRoot}
	class={flush
		? 'flex flex-col w-full h-full'
		: 'flex flex-col w-[min(420px,calc(100vw-20px))] max-h-[60vh]'}
	onkeydown={handleSearchKeydown}
	onmousemove={() => (mouseActive = true)}
>
	{#if externalFilter === undefined}
		<div class="px-3 py-2 border-b border-gray-200 dark:border-gray-700">
			<TextInput
				bind:this={searchInput}
				bind:value={internalFilter}
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
	{/if}

	{#if scope.length > 0 && !isSearching}
		<button
			type="button"
			class="flex items-center gap-1.5 w-full text-left px-3 py-1 text-xs font-medium font-mono text-secondary bg-surface-secondary/20 hover:bg-surface-hover transition-colors"
			onmousedown={(e) => e.preventDefault()}
			onclick={goUp}
			title={headerLabel}
		>
			<ChevronLeft size={12} class="shrink-0 text-secondary" />
			{#if headerChain[0]}
				{@render defaultBranchIcon(headerChain[0])}
			{/if}
			<span bind:this={breadcrumbSpan} class="flex-1 min-w-0 truncate truncate-start">
				{headerLabelDisplay}
			</span>
		</button>
	{/if}

	<div class="flex-1 overflow-y-auto" role="listbox" id={listboxId}>
		{#if isSearching}
			{@const total = (searchedItems ?? []).length}
			{#if !searchedItems}
				<div role="status" class="px-3 py-2 text-xs text-tertiary flex items-center gap-2">
					<Loader2 size={14} class="animate-spin" /> Searching…
				</div>
			{:else if total === 0}
				<div role="status" class="px-3 py-2 text-xs text-tertiary">No matches</div>
			{:else}
				{#each searchResultsByGroup as { group, items } (group?.key ?? '__none')}
					{#if group}
						<div class="px-3 pt-3 pb-1 text-2xs uppercase tracking-wide text-tertiary font-medium">
							{group.label}
						</div>
					{/if}
					<ul class="pb-1">
						{#each items as r (r.leaf.key)}
							<li>{@render leafRow(r.leaf, r.leaf.secondary ?? r.leaf.label, 'py-1.5')}</li>
						{/each}
					</ul>
				{/each}
			{/if}
		{:else if branchLoading && entryList.length === 0}
			<div role="status" class="px-3 py-2 text-xs text-tertiary flex items-center gap-2">
				<Loader2 size={14} class="animate-spin" /> Loading…
			</div>
		{:else if entryList.length === 0}
			<div role="status" class="px-3 py-2 text-xs text-tertiary">Empty</div>
		{:else}
			<div class="flex flex-col py-1">
				{#each entryList as entry (entry.key)}
					{@const isHl = entry.key === highlightedKey}
					{#if entry.type === 'leaf'}
						{@render leafRow(
							entry.node,
							leafSecondary?.(entry.node, scope) ?? entry.node.secondary,
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
							{@render defaultBranchIcon(entry.node)}
							<span class="flex-1 truncate">{entry.node.label}</span>
							{#if entry.node.loading}
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
	/* Path truncates from the start (left ellipsis) so the deepest
	 * (rightmost) folder stays visible. `unicode-bidi: plaintext` keeps
	 * each path segment laid out per its own direction. */
	.truncate-start {
		direction: rtl;
		text-align: left;
		unicode-bidi: plaintext;
	}
</style>
