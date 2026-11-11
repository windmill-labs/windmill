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
	import { CheckCircle2, ChevronLeft, ChevronRight, Loader2 } from 'lucide-svelte'
	import { fade, fly } from 'svelte/transition'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { createFloatingActions } from 'svelte-floating-ui'
	import { flip, offset, shift } from 'svelte-floating-ui/dom'
	import { generateRandomString } from '$lib/utils'
	import { onMount, untrack, type Snippet } from 'svelte'
	import {
		collectLeavesGrouped,
		leafHaystack,
		resolveScope,
		scopeChain,
		type DrillBranch,
		type DrillIcon,
		type DrillLeaf,
		type DrillLeafAction,
		type DrillNode
	} from './drillPicker'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { BadgeColor } from '$lib/components/common/badge/model'
	import Button from '$lib/components/common/button/Button.svelte'
	import { isNewTabClick } from '$lib/navigation'

	interface Props {
		tree: DrillNode<L>[]
		/** Picking a leaf: a click, or Enter on its highlighted row. Without it, a click on a leaf
		 * does nothing and Enter runs the highlighted row's selected `leafActions` button, for a
		 * picker whose rows act only through their buttons. */
		onPick?: (leaf: DrillLeaf<L>) => void
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
		/** Loading indicator for the ROOT level (scope `[]`), which has no
		 * branch to carry a `loading` flag. Needed by flat workspace layouts
		 * whose root entries only exist once items are fetched. */
		rootLoading?: boolean
		/** Full-text hover tooltip for a leaf row (rows truncate their text).
		 * Return `undefined` to show none for that leaf. */
		rowTooltip?: (leaf: DrillLeaf<L>) => string | undefined
		/** A second thing a leaf row can do, offered on hover and on ⌘/Ctrl-pick
		 * (the preview picker's "open the editor instead of the deployed page").
		 * Return `undefined` for a leaf that has only one. The button renders as a
		 * SIBLING of the row — a button inside a button is invalid markup — and
		 * stays out of the tab order, because focus lives in the search field and
		 * rows are reached with the arrow keys. */
		leafAction?: (
			leaf: DrillLeaf<L>
		) => { icon: DrillIcon; label: string; title: string; run: () => void } | undefined
		/** Status badge on the right of a leaf row. Returns `undefined` for a leaf with
		 * nothing to report. The hover action, when there is one, covers it. With `leafActions` it
		 * is plain text and `color` is unused. */
		leafBadge?: (leaf: DrillLeaf<L>) => { label: string; color: BadgeColor } | undefined
		/** Where picking a leaf navigates, for a host whose pick is a navigation. The row then
		 * renders as a link: a plain click still goes through `onPick`, while a middle or
		 * ⌘/Ctrl-click opens it in a new tab — taking ⌘/Ctrl-click from `leafAction`, which
		 * keeps ⌘/Ctrl-Enter. */
		leafHref?: (leaf: DrillLeaf<L>) => string | undefined
		/** What a leaf row offers, laid over its end while the row is highlighted (hovered or reached
		 * with the arrow keys), so the row does not reflow: the `preferred` one (else the first) as a
		 * button, which Enter runs when there is no `onPick`, and the others in its dropdown. In
		 * place of `leafAction`, not with it. */
		leafActions?: (leaf: DrillLeaf<L>) => DrillLeafAction[] | undefined
		/** What a `leafActions` button just did on a leaf ("Read granted"), shown over the row's
		 * end, highlighted or not, while it returns a text. */
		leafDone?: (leaf: DrillLeaf<L>) => string | undefined
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
		onFilterChange,
		rootLoading = false,
		rowTooltip,
		leafAction,
		leafBadge,
		leafHref,
		leafActions,
		leafDone
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
		// Rows re-render under a stationary pointer without firing mouseleave,
		// which would strand the tooltip on a removed row.
		tooltipLeave()
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

	// Group filtered results for display: nearest `searchGroup` branch ancestor
	// first, the leaf's own `section` otherwise.
	const searchResultsByGroup = $derived.by(() => {
		const groups = new Map<
			string,
			{ key: string; label: string | null; items: (SearchEntry & { marked: string })[] }
		>()
		if (!searchedItems) return [] as { key: string; label: string | null; items: SearchEntry[] }[]
		for (const r of searchedItems) {
			const gkey = r.group?.key ?? (r.leaf.section ? `__section:${r.leaf.section}` : '__none')
			const existing = groups.get(gkey)
			if (existing) existing.items.push(r)
			else
				groups.set(gkey, {
					key: gkey,
					label: r.group?.label ?? r.leaf.section ?? null,
					items: [r]
				})
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

	/** Whether any row in this list has a second line, which sets every row's height. */
	const entriesTwoLine = $derived(entryList.some((e) => e.type === 'leaf' && !!e.node.secondary))

	// Browse rows annotated with the section header rendered above them. A
	// header is emitted when a leaf's section differs from the previous LEAF's
	// section — comparing against the previous entry of any type would insert a
	// duplicate header after every branch interleaved between same-section leaves.
	const entryRows = $derived.by(() => {
		let lastLeafSection: string | undefined
		return entryList.map((entry) => {
			const section = entry.type === 'leaf' ? entry.node.section : undefined
			const header = entry.type === 'leaf' && section !== lastLeafSection ? section : undefined
			if (entry.type === 'leaf') lastLeafSection = section
			return { entry, header }
		})
	})

	// Search nav must follow DISPLAY order (the grouped blocks), not the raw
	// fuzzy ranking — grouping reorders interleaved results, and rank-ordered
	// keys would make ArrowUp/Down jump between non-adjacent visible rows.
	const navKeys = $derived(
		isSearching
			? searchResultsByGroup.flatMap((g) => g.items.map((r) => r.leaf.key))
			: entryList.map((e) => e.key)
	)

	let highlightedKey = $state<string | undefined>(untrack(() => initialHighlight))
	let highlightedId = $derived(highlightedKey ? idFor(highlightedKey) : undefined)

	// Visible leaf flagged `current`, if any. When the highlight's key vanishes
	// (rows replaced under it — e.g. the workspace tree swapping a synthetic
	// current-item leaf for the loaded storage-keyed row, whose key differs),
	// this is the right anchor to fall back to: resetting to the first row
	// would make Enter silently target an unrelated sibling.
	const currentLeafKey = $derived(
		isSearching
			? (searchedItems ?? ([] as typeof searchItems)).find((r) => r.leaf.current)?.leaf.key
			: entryList.find((e) => e.type === 'leaf' && e.node.current)?.key
	)

	$effect(() => {
		if (navKeys.length === 0) return
		if (!highlightedKey || !navKeys.includes(highlightedKey)) {
			highlightedKey = currentLeafKey ?? navKeys[0]
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

	function leafForKey(key: string): DrillLeaf<L> | undefined {
		if (isSearching) return (searchedItems ?? []).find((r) => r.leaf.key === key)?.leaf
		const entry = entryList.find((e) => e.key === key)
		return entry?.type === 'leaf' ? entry.node : undefined
	}

	function moveHighlight(delta: 1 | -1) {
		if (navKeys.length === 0) return
		const cur = navKeys.indexOf(highlightedKey ?? '')
		const next = cur < 0 ? 0 : (cur + delta + navKeys.length) % navKeys.length
		highlightedKey = navKeys[next]
		mouseActive = false
		requestAnimationFrame(() => {
			scrollHighlightIntoView()
			// Keyboard parity for the tooltip: anchor it to the row the highlight
			// landed on — otherwise the full descriptions are pointer-only.
			if (!rowTooltip || !highlightedKey) return
			const leaf = leafForKey(highlightedKey)
			const el = pickerRoot?.querySelector<HTMLElement>(
				`[data-nav-key="${CSS.escape(highlightedKey)}"]`
			)
			if (leaf && el) tooltipEnter(el, leaf)
			else tooltipLeave()
		})
	}

	function setHoverHighlight(key: string) {
		// Ignored until the user actually moves the mouse. Prevents the
		// cursor (parked over a row) from clobbering keyboard-driven
		// selection when the layout shifts beneath it.
		// A row's open menu sits over the rows below it, and the pointer crosses them to reach it:
		// moving the highlight would take the menu's button away.
		if (mouseActive && !actionMenuOpen) highlightedKey = key
	}

	/** `alt` picks the row's second action (see `leafAction`) rather than the row
	 * itself — ⌘/Ctrl-click and ⌘/Ctrl-Enter, the keyboard reach for the button
	 * that only appears on hover. A leaf with no second action ignores it. */
	function pick(leaf: DrillLeaf<L>, alt = false) {
		if (leaf.disabled) return
		if (!onPick) {
			if (leaf.key === highlightedKey) runMainAction()
			return
		}
		// The second action is the row's other destination (its editor, for the deployed page
		// this tab shows), so it stays available on the current row the plain pick skips.
		if (alt) {
			const action = leafAction?.(leaf)
			if (action) {
				action.run()
				return
			}
		}
		if (leaf.current) return
		onPick(leaf)
	}

	function runMainAction() {
		const action = mainAction
		if (action && !action.disabled && !action.loading) runAction(action)
	}

	/** The button area's width when each row's action ran, the least its success pill takes. Read
	 * from `actionsWidth` live, the pill would resize on hover, which moves the highlighted row. */
	let pillWidths: Record<string, number> = $state({})
	function runAction(action: DrillLeafAction) {
		if (highlightedKey) pillWidths[highlightedKey] = actionsWidth
		action.run()
	}

	/** Fades out what lies under the highlighted row's buttons, which sit over its end. */
	function fadeUnder(width: number): string {
		const mask = `linear-gradient(to left, transparent ${width}px, #000 ${width + 16}px)`
		return `mask-image: ${mask}; -webkit-mask-image: ${mask}`
	}

	// Single shared hover tooltip (opt-in via `rowTooltip`): one floating node
	// re-anchored to the hovered row, instead of a popover per row — rows can
	// number in the hundreds. Portaled to body because the picker often lives
	// inside an overflow-clipped floating container that would cut it off.
	let tooltipText = $state<string | undefined>(undefined)
	let tooltipTimer: ReturnType<typeof setTimeout> | undefined
	const [tooltipRef, tooltipContent] = createFloatingActions({
		strategy: 'fixed',
		placement: 'right-start',
		middleware: [offset(8), flip(), shift({ padding: 8 })],
		autoUpdate: true
	})

	function tooltipEnter(el: HTMLElement, leaf: DrillLeaf<L>) {
		if (!rowTooltip) return
		clearTimeout(tooltipTimer)
		const text = rowTooltip(leaf)
		if (!text) {
			tooltipText = undefined
			return
		}
		tooltipTimer = setTimeout(() => {
			tooltipRef(el)
			tooltipText = text
		}, 250)
	}

	function tooltipLeave() {
		clearTimeout(tooltipTimer)
		tooltipText = undefined
	}

	$effect(() => () => clearTimeout(tooltipTimer))

	function activate(key: string | undefined, alt = false) {
		if (!key) return
		if (isSearching) {
			const found = (searchedItems ?? []).find((r) => r.leaf.key === key)
			if (found) pick(found.leaf, alt)
			return
		}
		const entry = entryList.find((e) => e.key === key)
		if (!entry) return
		drill(entry, alt)
	}

	function drill(entry: Entry, alt = false) {
		if (entry.type === 'branch') {
			scope = [...scope, entry.key]
		} else {
			pick(entry.node, alt)
		}
	}

	function goUp() {
		if (scope.length === 0) return
		const leaving = scope[scope.length - 1]
		scope = scope.slice(0, -1)
		highlightedKey = leaving
	}

	function handleSearchKeydown(e: KeyboardEvent) {
		// A row's button has focus only after a click: it keeps its own keys, and Enter must not
		// also reach the row. ↑/↓ and Escape return to the search field.
		if ((e.target as HTMLElement | null)?.closest?.('[data-drill-action]')) {
			if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Escape') {
				e.preventDefault()
				e.stopPropagation()
				focus()
				if (e.key !== 'Escape') moveHighlight(e.key === 'ArrowDown' ? 1 : -1)
			}
			return
		}
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
			// Without `onPick`, Enter on a leaf runs its selected button, which Tab must not do.
			if (e.key === 'Tab' && !onPick && leafForKey(highlightedKey!)) return
			e.preventDefault()
			e.stopPropagation()
			mouseActive = false
			activate(highlightedKey, e.metaKey || e.ctrlKey)
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

	const branchLoading = $derived(currentBranch ? (currentBranch.loading ?? false) : rootLoading)

	const highlightedNode = $derived.by<DrillNode<L> | undefined>(() => {
		if (!highlightedKey) return undefined
		if (isSearching) return (searchedItems ?? []).find((r) => r.leaf.key === highlightedKey)?.leaf
		return entryList.find((e) => e.key === highlightedKey)?.node
	})

	const highlightedActions = $derived(
		highlightedNode?.type === 'leaf' && !highlightedNode.disabled
			? (leafActions?.(highlightedNode) ?? [])
			: []
	)
	const mainAction = $derived(highlightedActions.find((a) => a.preferred) ?? highlightedActions[0])
	const otherActions = $derived(highlightedActions.filter((a) => a !== mainAction))
	let actionMenuOpen = $state(false)

	/** Answers an Escape a host takes before the picker can: closes a row's dropdown, which hands
	 * focus back to its chevron, or leaves a row's button for the search field. False when neither
	 * applies, so the host closes instead. */
	export function escape(target: EventTarget | null): boolean {
		if (actionMenuOpen) {
			actionMenuOpen = false
			return true
		}
		if (!(target as HTMLElement | null)?.closest?.('[data-drill-action]')) return false
		focus()
		return true
	}
	let actionsWidth = $state(0)
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

{#snippet leafRow(
	leaf: DrillLeaf<L>,
	secondary: string | undefined,
	baseClass: string,
	twoLineHeight: boolean
)}
	{@const key = leaf.key}
	{@const isHl = key === highlightedKey}
	{@const isCur = !!leaf.current}
	{@const tip = rowTooltip?.(leaf)}
	{@const action = leaf.disabled ? undefined : leafAction?.(leaf)}
	{@const badge = leafBadge?.(leaf)}
	{@const href = leaf.disabled ? undefined : leafHref?.(leaf)}
	{@const inert = !onPick}
	{@const actions = isHl ? highlightedActions : []}
	{@const done = leafDone?.(leaf)}
	<!-- The row's action sits BESIDE the option rather than inside it: a button nested in
	     a button is invalid markup. The wrapper carries the row's padding, highlight and
	     hover group so the two still read as one row. -->
	<!-- Hover is tracked on the wrapper, not on the option button: the row's action is a
	     sibling of that button, so entering it would otherwise read as leaving the row and
	     drop the highlight out from under the pointer. -->
	<!-- A disabled option dims through `disabled:opacity-50` and shows its cursor on this wrapper:
	     the base `button:disabled` rule in tailwind.config.cjs outranks a plain `opacity-50` and
	     turns pointer events off on the button itself. -->
	<!-- `leafActions` show only on the highlighted row, so a pointer that comes back after the
	     arrow keys moved the highlight takes it on its first move, not on the next row it enters. -->
	<div
		class="group relative flex items-center transition-colors pr-3 {baseClass} {isHl
			? 'bg-surface-hover'
			: ''} {leaf.disabled ? 'cursor-not-allowed' : ''}"
		onmouseenter={(e) => {
			setHoverHighlight(key)
			tooltipEnter(e.currentTarget as HTMLElement, leaf)
		}}
		onmousemove={leafActions && !isHl ? () => setHoverHighlight(key) : undefined}
		onmouseleave={tooltipLeave}
	>
		<!-- With `leafActions`, what fades out under the row's buttons; `contents` otherwise. -->
		<div
			class={leafActions ? 'min-w-0 flex-1 flex items-center' : 'contents'}
			style={actions.length ? fadeUnder(actionsWidth) : undefined}
		>
			<svelte:element
				this={href ? 'a' : inert ? 'div' : 'button'}
				type={href || inert ? undefined : 'button'}
				{href}
				id={idFor(key)}
				role="option"
				aria-selected={isHl}
				aria-disabled={inert && leaf.disabled ? 'true' : undefined}
				data-nav-key={key}
				aria-current={isCur ? 'true' : undefined}
				aria-label={tip ? `${leaf.label} — ${tip}` : undefined}
				class="min-w-0 flex-1 text-left flex items-center gap-2 pl-3 {isCur
					? 'cursor-default text-emphasis font-medium'
					: 'text-[color:inherit]'} {inert
					? leaf.disabled
						? 'opacity-50'
						: 'cursor-default'
					: 'disabled:opacity-50'}"
				disabled={href || inert ? undefined : leaf.disabled}
				tabindex={inert ? -1 : 0}
				onmousedown={(e: MouseEvent) => e.preventDefault()}
				onclickcapture={inert
					? undefined
					: (e: MouseEvent) => {
							// Capture, not a delegated onclick: the picker is portalled outside the app root,
							// where a delegated handler runs after SvelteKit's link handler has already
							// followed the href, too late for preventDefault to keep a plain click on `pick`.
							if (href && isNewTabClick(e)) return
							e.preventDefault()
							pick(leaf, e.metaKey || e.ctrlKey)
						}}
			>
				{@render defaultLeafIcon(leaf)}
				<!-- Centred in a two-line row's height so a one-line leaf sits where a two-line
				     one's pair does, rather than riding high against the top. A list with no
				     second line anywhere (a picker's pages) keeps its rows one line tall. -->
				<div
					class="min-w-0 flex-1 flex flex-col justify-center {twoLineHeight
						? 'min-h-[2.25rem]'
						: ''}"
				>
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
			</svelte:element>
			{#if leafActions}
				<!-- Plain text beside the row's buttons, which sit over it while the row is
				     highlighted: a filled badge reads as one more button. -->
				{#if badge}
					<span class="shrink-0 pl-2 text-xs text-secondary">{badge.label}</span>
				{/if}
				<!-- Keyed on the props rather than on this leaf's own badge or action, so a picker
				     that has either holds the column open on every row: paths then truncate at one
				     column and the button lands in the same place down the list. -->
			{:else if leafBadge || leafAction}
				<!-- Both sit in one grid cell so they cross-fade in place rather than sitting
				     side by side or shifting the row as one replaces the other. -->
				<div class="shrink-0 w-[4.5rem] grid justify-items-end items-center pl-2">
					{#if action}
						<!-- `invisible`, not `hidden`: display cannot be transitioned, while
						     visibility both fades with opacity and keeps the button out of the tab
						     order while it is out of sight — the picker's focus belongs to its
						     search field, and rows are reached with the arrow keys.
						     The classes sit on this wrapper, not on the Button: `wrapperClasses` is
						     twMerged with the button's own `flex`, which drops a conflicting
						     display and leaves it showing beside the badge. -->
						<div
							class="col-start-1 row-start-1 invisible opacity-0 transition-[opacity,visibility] duration-150 group-hover:visible group-hover:opacity-100"
						>
							<Button
								unifiedSize="xs"
								variant="default"
								title={action.title}
								startIcon={{ icon: action.icon }}
								onClick={action.run}
								wrapperClasses="shrink-0"
							>
								{action.label}
							</Button>
						</div>
					{/if}
					{#if badge}
						<div
							class="col-start-1 row-start-1 transition-[opacity,visibility] duration-150 {action
								? 'group-hover:invisible group-hover:opacity-0'
								: ''}"
						>
							<Badge small color={badge.color}>{badge.label}</Badge>
						</div>
					{/if}
				</div>
			{/if}
		</div>
		{#if actions.length}
			<!-- Over the row's end, so a row keeps its layout whether or not it shows them. -->
			<div
				class="absolute inset-y-0 right-3 flex items-center gap-1"
				bind:clientWidth={actionsWidth}
			>
				{#if mainAction && !done}
					<!-- Local, so it waits for the pill to leave but not for a hover. -->
					<span data-drill-action in:fade={{ delay: 150, duration: 100 }}>
						<Button
							unifiedSize="sm"
							variant="default"
							startIcon={mainAction.icon ? { icon: mainAction.icon } : undefined}
							loading={mainAction.loading}
							disabled={mainAction.disabled}
							onClick={() => mainAction && runAction(mainAction)}
							dropdownItems={otherActions.map((a) => ({
								label: a.label,
								description: a.hint,
								icon: a.icon,
								disabled: a.disabled || a.loading,
								onClick: () => runAction(a)
							}))}
							bind:dropdownOpen={actionMenuOpen}
						>
							{mainAction.label}
						</Button>
					</span>
				{/if}
			</div>
		{/if}
		{#if done}
			<!-- SaveButton's success pill, over the row's button and its dropdown chevron. -->
			<div
				class="absolute inset-y-0 my-auto h-7 right-3 z-10 flex items-center justify-center gap-1.5 px-2 rounded-md text-xs font-medium whitespace-nowrap bg-green-200 dark:bg-green-800 text-green-800 dark:text-green-200"
				style={pillWidths[leaf.key] ? `min-width: ${pillWidths[leaf.key]}px` : undefined}
				role="status"
				transition:fly={{ y: -10, duration: 150 }}
			>
				<CheckCircle2 size={14} class="shrink-0" />
				{done}
			</div>
		{/if}
	</div>
{/snippet}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={pickerRoot}
	class={flush
		? 'flex flex-col w-full h-full'
		: 'flex flex-col w-[min(420px,calc(100vw-20px))] max-h-[60vh] min-h-0'}
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
				{#each searchResultsByGroup as { key, label, items } (key)}
					{@const twoLineHeight = items.some((r) => !!r.leaf.secondary)}
					{#if label}
						<div
							class="px-3 pt-3 pb-1 mt-2 first:mt-0 text-3xs uppercase tracking-wide text-hint font-medium"
						>
							{label}
						</div>
					{/if}
					<ul class="pb-1">
						{#each items as r (r.leaf.key)}
							<li>
								{@render leafRow(r.leaf, r.leaf.secondary ?? r.leaf.label, 'py-1.5', twoLineHeight)}
							</li>
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
				{#each entryRows as { entry, header } (entry.key)}
					{@const isHl = entry.key === highlightedKey}
					{#if header}
						<div
							class="px-3 pt-2 pb-1 mt-2 first:mt-0 text-3xs uppercase tracking-wide text-hint font-medium"
						>
							{header}
						</div>
					{/if}
					{#if entry.type === 'leaf'}
						{@render leafRow(
							entry.node,
							leafSecondary?.(entry.node, scope) ?? entry.node.secondary,
							'py-1.5',
							entriesTwoLine
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
				{#if branchLoading}
					<!-- More rows are still being fetched (flat-layout roots surface
					     entries only as their kinds load — e.g. Pages present, dirs
					     pending). The empty case is handled above. -->
					<div role="status" class="px-3 py-1.5 text-xs text-tertiary flex items-center gap-2">
						<Loader2 size={14} class="animate-spin" /> Loading…
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

{#if tooltipText}
	<Portal target="body">
		<div
			use:tooltipContent
			class="max-w-72 rounded-md border bg-surface px-2.5 py-1.5 text-xs text-primary shadow-md whitespace-pre-wrap break-words"
			style="z-index: {zIndexes.tooltip};"
			role="tooltip"
		>
			{tooltipText}
		</div>
	</Portal>
{/if}

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
