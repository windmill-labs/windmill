<script lang="ts">
	import { buildDiffTree, folderKeyFor, type AppRootMeta, type TreeNode } from './diffTree'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import {
		Check,
		CheckCircle2,
		ChevronDown,
		ChevronRight,
		ExternalLink,
		Folder,
		Loader2,
		Save,
		User
	} from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import WorkspaceItemRow from '$lib/components/WorkspaceItemRow.svelte'
	import WorkspaceItemDiffViewer from '$lib/components/WorkspaceItemDiffViewer.svelte'
	import {
		rawAppDiffToItems,
		type RawAppish,
		type RawAppFileItem,
		type RawAppRunnableItem,
		type RawAppSyntheticItem
	} from '$lib/components/raw_apps/rawAppDiffUtils'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { DiffIcon, SquareSplitHorizontal } from 'lucide-svelte'
	import { tick, untrack } from 'svelte'
	import { fade, fly, slide } from 'svelte/transition'
	import { cubicOut } from 'svelte/easing'
	import type { Snippet } from 'svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import ExternalEditLink from '../ExternalEditLink.svelte'
	import { actionFor, badgeOf, type DeployItem } from './sessionDeployModel'
	import type { SessionDeployModel, DiffValues } from './sessionDeployModel.svelte'

	// The session "Edits" surface. Reads a unified item model (the session's
	// drafts, scoped to what this session edited) and renders a folder tree
	// (left) plus a scroll-through diff column with per-block sticky headers
	// (right). Deploy is granular per-item; batch/PR flows and fork→parent
	// promotion are handed to the compare page via the footer link.
	let {
		model,
		title,
		editUrlFor = undefined,
		titleExtra,
		compareSessionHref,
		workspaceLabel,
		successHoldMs = 1200
	}: {
		model: SessionDeployModel
		title: string
		/** Editor URL for a row (opens in the workspace the item lives in). */
		editUrlFor?: (item: DeployItem) => string | undefined
		titleExtra?: Snippet
		/** Display name of the workspace deploys land in — the deployed tick's
		 *  hover reads "Deployed in <label>". */
		workspaceLabel?: string
		/** How long the post-deploy check beat holds before the row flips. */
		successHoldMs?: number
		/** Compare page for this session's edits (footer) — preselected via
		 *  `from_session`. Where "deploy all / request review" happens. */
		compareSessionHref?: string
	} = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let searchQuery = $state('')
	let diffStyle = $state<'sbs' | 'inline'>('sbs')
	const inlineDiff = $derived(diffStyle === 'inline')

	// The sidebar's inner right padding must ADAPT to the reserved scrollbar
	// gutter (platform-dependent: 0 for overlay scrollbars, ~15px classic) so
	// gutter + padding always totals the same 6px inset the left side uses.
	// Pure CSS can't express "6px minus the gutter", hence the measurement.
	function padRightForGutter(node: HTMLElement) {
		const apply = () => {
			const gutter = node.offsetWidth - node.clientWidth
			node.style.paddingRight = `${Math.max(0, 6 - gutter)}px`
		}
		apply()
		const ro = new ResizeObserver(apply)
		ro.observe(node)
		return { destroy: () => ro.disconnect() }
	}

	// Discard is destructive (a draft-only item disappears entirely) — confirm.
	const discardConfirm = createAsyncConfirmationModal()
	async function confirmDiscard(item: DeployItem) {
		const confirmed = await discardConfirm.ask({
			title: 'Discard draft?',
			confirmationText: 'Discard draft',
			children: item.draftOnly
				? `${item.displayPath} was never deployed — discarding its draft deletes it entirely.`
				: `The draft changes on ${item.displayPath} will be permanently discarded; the deployed version is kept.`
		})
		if (confirmed) void model.discardRow(item)
	}

	// Post-deploy staging (same pattern as SaveButton): on success, the row keeps
	// rendering its pre-deploy snapshot while a green check flies in over the
	// Deploy button; after the hold it releases, letting the collapse + badge
	// transitions play on the real (deployed) state.
	let staged = $state<Record<string, DeployItem>>({})
	let stagedTimers: ReturnType<typeof setTimeout>[] = []
	$effect(() => {
		return () => stagedTimers.forEach(clearTimeout)
	})
	async function deployStaged(item: DeployItem) {
		if (!(await model.deployRow(item))) return
		staged = { ...staged, [item.key]: item }
		stagedTimers.push(
			setTimeout(() => {
				const next = { ...staged }
				delete next[item.key]
				staged = next
			}, successHoldMs)
		)
	}

	export function open() {
		loadedDiffs = {}
		mountedRows = {}
		folderOpen = {}
		staged = {}
		model.load()
		drawer?.openDrawer()
		setTimeout(() => searchInputEl?.focus(), 50)
	}

	// ── Badge presentation ─────────────────────────────────────────────────────
	// One footprint for both status pills (draft / deployed) so they line up at a
	// compact, uniform size.
	const STATUS_BADGE_CLASS = 'px-1.5 py-0.5 gap-0.5 text-2xs'

	// ── Rendered list: all session items, then text-searched (no filter tabs) ──
	const segmentItems = $derived(model.items)

	// A tree row is either a real deploy item or — once its raw app's values
	// have loaded — a synthetic per-file/per-runnable item: the app's leaf row
	// is replaced by its files, and the folder at the app's path becomes the app
	// root (tagged via appRootMeta). Synthetics carry `appPath`.
	type DisplayEntry = DeployItem | RawAppSyntheticItem
	function isSynthetic(e: DisplayEntry): e is RawAppSyntheticItem {
		return 'appPath' in e
	}
	// Synthetic keys are prefixed so they can't collide with a real item that
	// happens to live at `<appPath>/runnables/foo`.
	function displayKey(e: DisplayEntry): string {
		return isSynthetic(e) ? `rawapp:${e.kind}/${e.path}` : e.key
	}
	function displayPathOf(e: DisplayEntry): string {
		return e.displayPath ?? e.path
	}

	// Raw-app storage path → its deploy item (owning-app lookup for synthetics).
	const appByPath = $derived(
		new Map(segmentItems.filter((d) => d.deployKind === 'raw_app').map((d) => [d.path, d]))
	)

	const displayEntries = $derived.by(() => {
		const out: DisplayEntry[] = []
		for (const d of segmentItems) {
			if (d.deployKind === 'raw_app') {
				const loaded = loadedDiffs[d.key]
				if (loaded?.state === 'ready') {
					// A deployed (done) app has no changed files to diff — unwrap it
					// into its full current contents instead (one-sided → every file
					// listed; rows are plain so the 'added' status never shows).
					const subs = d.done
						? rawAppDiffToItems(
								d.path,
								undefined,
								loaded.after as RawAppish | undefined,
								d.displayPath
							)
						: rawAppItems(d, loaded)
					if (subs.length > 0) {
						out.push(...subs)
						continue
					}
				}
			}
			out.push(d)
		}
		return out
	})

	function searchableText(e: DisplayEntry): string {
		if (isSynthetic(e)) return [displayPathOf(e), e.kind].join(' ')
		return [e.displayPath, e.deployKind, e.summary ?? ''].join(' ')
	}
	let searchedEntries: (DisplayEntry & { marked?: string })[] | undefined = $state(undefined)
	const searchActive = $derived(searchQuery.trim().length > 0)
	const filteredEntries = $derived.by(() => {
		if (!searchActive) return displayEntries
		return (searchedEntries ?? []) as DisplayEntry[]
	})

	// File names inside unexpanded apps don't exist client-side until the app's
	// values load — a live search loads every raw app so matches are complete.
	$effect(() => {
		if (!searchActive) return
		const items = segmentItems
		untrack(() => {
			for (const d of items) if (d.deployKind === 'raw_app') void loadDiffFor(d)
		})
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
			filteredEntries.map((e) => ({
				key: displayKey(e),
				structurePath: displayPathOf(e),
				data: e
			})),
			appRootMeta
		)
	)

	// Content blocks follow the sidebar's order: walk the tree (all folders open,
	// so collapsing the sidebar never hides a block) and emit each owning deploy
	// item once, so the diff column top-to-bottom matches the tree.
	const orderedItems = $derived.by(() => {
		const seen = new Set<string>()
		const out: DeployItem[] = []
		for (const entry of treeModel.order(() => true)) {
			if (entry.type !== 'file') continue
			const item = isSynthetic(entry.data) ? appByPath.get(entry.data.appPath) : entry.data
			if (item && !seen.has(item.key)) {
				seen.add(item.key)
				out.push(item)
			}
		}
		return out
	})

	function rowId(d: DeployItem): string {
		return `ws-diff-${d.key}`
	}

	// Keep the column's pt-3 breathing room when a click scrolls a card to the top.
	const SCROLL_TOP_INSET = 12

	// Smooth-scroll the row's card near the top (leaving the column's top inset).
	// Native scrollIntoView lands short here because lazily-mounted diffs grow the
	// list mid-scroll; instead we ease toward the target and re-measure it every
	// frame, so the animation stays smooth while still landing flush once the
	// layout settles. Bails on user scroll.
	function scrollToDiff(d: DeployItem) {
		const el = document.getElementById(rowId(d))
		const container = mainScrollEl
		if (!el || !container) return
		let frames = 0
		let settled = 0
		let cancelled = false
		const cancel = () => (cancelled = true)
		container.addEventListener('wheel', cancel, { once: true, passive: true })
		container.addEventListener('touchstart', cancel, { once: true, passive: true })
		const teardown = () => {
			container.removeEventListener('wheel', cancel)
			container.removeEventListener('touchstart', cancel)
		}
		const step = () => {
			if (cancelled) return teardown()
			const off =
				el.getBoundingClientRect().top - container.getBoundingClientRect().top - SCROLL_TOP_INSET
			if (Math.abs(off) < 1) {
				if (++settled >= 2) return teardown()
			} else {
				settled = 0
				// Ease-out for the long haul; snap the final approach so it settles fast.
				container.scrollTop += Math.abs(off) < 20 ? off : Math.round(off * 0.22)
			}
			if (frames++ < 120) requestAnimationFrame(step)
			else teardown()
		}
		step()
	}

	// Transient ring pulse on the card a tree click just revealed, so the eye
	// lands on the right block after the scroll.
	let flashKey = $state<string | undefined>(undefined)
	let flashTimer: ReturnType<typeof setTimeout> | undefined
	function flashRow(key: string) {
		flashKey = key
		clearTimeout(flashTimer)
		flashTimer = setTimeout(() => (flashKey = undefined), 800)
	}

	function revealDiff(d: DeployItem, key?: string) {
		if (key) highlightedKey = key
		flashRow(d.key)
		scrollToDiff(d)
	}

	// A file's sub-diff anchor lives inside the app's main-panel block, which
	// mounts lazily on scroll — force-mount it before scrolling to the anchor.
	// A deployed app's block has no per-file sections ("no pending changes"), so
	// fall back to the app block itself.
	async function revealSynthetic(s: RawAppSyntheticItem, key?: string) {
		if (key) highlightedKey = key
		const app = appByPath.get(s.appPath)
		if (!app) return
		flashRow(app.key)
		if (!mountedRows[app.key]) {
			mountedRows[app.key] = true
			await tick()
		}
		const target =
			document.getElementById(`ws-diff-${displayKey(s)}`) ?? document.getElementById(rowId(app))
		target?.scrollIntoView({ behavior: 'smooth', block: 'start' })
	}

	// ── Keyboard nav ─────────────────────────────────────────────────────────
	let folderOpen: Record<string, boolean> = $state({})
	// Folders default open, raw-app roots default collapsed; a live search
	// forces everything open so matches inside apps are visible.
	const appFolderKeys = $derived(new Set([...appRootMeta.keys()].map((p) => folderKeyFor(p))))
	function isFolderOpen(key: string): boolean {
		if (searchActive) return true
		return folderOpen[key] ?? !appFolderKeys.has(key)
	}

	// Expand a still-unloaded raw app from its leaf row: mark the future app
	// folder open and fetch the values; once ready, the leaf is replaced by the
	// app root + file children. Re-expanding after a failure retries the load.
	function expandApp(d: DeployItem) {
		const fkey = folderKeyFor(d.displayPath)
		folderOpen[fkey] = true
		if (loadedDiffs[d.key]?.state === 'error') delete loadedDiffs[d.key]
		const wasHighlighted = highlightedKey === d.key
		void loadDiffFor(d).then(() => {
			if (wasHighlighted) highlightedKey = fkey
		})
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
		if (entry.type === 'file') {
			if (isSynthetic(entry.data)) void revealSynthetic(entry.data)
			else void revealDiff(entry.data)
			return
		}
		// App roots activate like rows (reveal the diff); folding stays on the
		// chevron / ArrowRight. Plain folders keep Enter-to-toggle.
		const appItem = entry.node.app
			? segmentItems.find((d) => d.key === entry.node.app?.summaryKey)
			: undefined
		if (appItem) revealDiff(appItem)
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
			if (!entry) return
			if (entry.type === 'file') {
				// An unloaded raw app is still a leaf — ArrowRight expands it.
				if (!isSynthetic(entry.data) && entry.data.deployKind === 'raw_app') {
					e.preventDefault()
					expandApp(entry.data)
				}
				return
			}
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
	// Visible height of the scroll area, used to size the trailing spacer so the
	// last item can always be scrolled until its header reaches the top.
	let mainHeight = $state(0)
	// The trailing spacer only needs to cover the gap the last item can't fill on
	// its own: viewport − lastItemHeight (0 when the item already fills the view).
	// Reserving the full viewport would let you scroll a whole item-height into
	// blank space past the end. A ResizeObserver keeps it right as the row grows.
	let lastRowHeight = $state(0)
	$effect(() => {
		const last = orderedItems.at(-1)
		const el = last && mainScrollEl ? document.getElementById(rowId(last)) : undefined
		if (!el) {
			lastRowHeight = 0
			return
		}
		lastRowHeight = el.offsetHeight
		const ro = new ResizeObserver(() => (lastRowHeight = el.offsetHeight))
		ro.observe(el)
		return () => ro.disconnect()
	})
	const trailingSpace = $derived(Math.max(0, mainHeight - lastRowHeight))
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
	items={displayEntries}
	bind:filteredItems={searchedEntries}
	f={(e: DisplayEntry) => searchableText(e)}
/>

<!-- The single row badge. Everything here was edited by the AI this session,
     so the draft badge is a plain pill — no author avatars. -->
{#snippet rowBadge(item: DeployItem)}
	{#if badgeOf(item) === 'draft'}
		<Badge
			color="indigo"
			small
			class={STATUS_BADGE_CLASS}
			title={item.draftOnly ? 'Never deployed and is only a draft' : 'Is deployed and has a draft'}
		>
			{item.draftOnly ? 'Draft only' : 'Draft'}
		</Badge>
	{:else}
		<!-- Deployed is the quiet terminal state — an icon-only green badge; the
		     hover carries the words. Fades in when a row transitions on deploy. -->
		<span in:fade={{ duration: 200 }} class="inline-flex shrink-0">
			<Badge
				color="green"
				small
				class="px-1 py-0.5"
				title={`Deployed in ${workspaceLabel ?? 'this workspace'}`}
			>
				<Check class="w-3 h-3" aria-label="Deployed" />
			</Badge>
		</span>
	{/if}
{/snippet}

<!-- Failure only — while deploying the button itself carries a spinner, and a
     success flies the green check over the button (see deployStaged). -->
{#snippet deployFailed(item: DeployItem)}
	{@const s = model.statusOf(item.key)}
	{#if s?.status === 'failed'}
		<span class="inline-flex items-center gap-1" title={s.error}>
			<Badge color="red" small>Failed</Badge>
			<Button
				variant="subtle"
				unifiedSize="xs"
				disabled={model.deploying}
				onclick={() => deployStaged(item)}
			>
				Retry
			</Button>
		</span>
	{/if}
{/snippet}

{#snippet renderTreeNode(node: TreeNode<DisplayEntry>, depth: number)}
	{#if node.type === 'folder'}
		{@const isUserScope = node.isScope && node.name.startsWith('u/')}
		{@const fkey = node.key}
		{@const open = isFolderOpen(fkey)}
		{@const isHl = fkey === highlightedKey}
		<details
			{open}
			ontoggle={(e) => {
				// Record real user toggles only; skip the echo fired when the `open`
				// attribute is driven by state (search force-open, expandApp).
				const domOpen = (e.currentTarget as HTMLDetailsElement).open
				if (domOpen !== isFolderOpen(fkey)) folderOpen[fkey] = domOpen
			}}
			class="select-none"
		>
			{#if node.app}
				{@const appItem = segmentItems.find((it) => it.key === node.app?.summaryKey)}
				<!-- App root: the row click reveals the app's diff like any item row;
				     folding is chevron-only (preventDefault blocks the native toggle). -->
				<summary
					role="option"
					aria-selected={isHl}
					data-nav-key={fkey}
					onmouseenter={() => setHoverHighlight(fkey)}
					onclick={(e) => {
						e.preventDefault()
						if (appItem) revealDiff(appItem, fkey)
					}}
					title={node.fullPath}
					class="flex items-center gap-2 pl-3 pr-1 py-2 rounded-md cursor-pointer hover:bg-surface-hover list-none [&::-webkit-details-marker]:hidden tree-summary {isHl
						? 'bg-surface-hover'
						: ''}"
					style="padding-left: {depth * 12 + 8}px"
				>
					<RowIcon kind="raw_app" size={12} />
					<span
						class="flex-1 min-w-0 truncate text-xs font-normal text-primary {node.app.summary
							? ''
							: 'font-mono'}"
					>
						{node.app.summary ?? node.name}
					</span>
					{#if appItem}
						{@render rowBadge(staged[appItem.key] ?? appItem)}
					{/if}
					<button
						type="button"
						aria-expanded={open}
						title={open ? 'Hide files' : 'Show files'}
						class="w-5 h-5 flex items-center justify-center shrink-0 rounded hover:bg-surface-secondary"
						onclick={(e) => {
							e.preventDefault()
							e.stopPropagation()
							folderOpen[fkey] = !open
						}}
					>
						<ChevronDown class="w-3 h-3 shrink-0 text-tertiary tree-chevron-open" />
						<ChevronRight class="w-3 h-3 shrink-0 text-tertiary tree-chevron-closed" />
					</button>
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
		     highlight starts at the row content instead of bleeding across the indent.
		     Every row ends with a w-5 slot (chevron or spacer) to keep badges aligned
		     with the folder rows' chevrons. -->
		<div class="flex items-stretch" style="padding-left: {depth * 12 + 4}px">
			{#if isSynthetic(d)}
				<WorkspaceItemRow
					kind={d.kind as any}
					iconPath={d.path}
					baseClass="py-2 min-w-0 pr-1 pl-1 rounded-md"
					singleLine
					secondary={node.name}
					highlighted={key === highlightedKey}
					navKey={key}
					indent={0}
					title={displayPathOf(d)}
					onclick={() => void revealSynthetic(d, key)}
					onmouseenter={() => setHoverHighlight(key)}
				>
					{#snippet extras()}
						<span class="w-5 shrink-0"></span>
					{/snippet}
				</WorkspaceItemRow>
			{:else}
				<WorkspaceItemRow
					kind={d.deployKind as any}
					iconPath={d.path}
					baseClass="py-2 min-w-0 pr-1 pl-1 rounded-md"
					singleLine
					summary={d.summary}
					secondary={node.name}
					highlighted={key === highlightedKey}
					navKey={key}
					indent={0}
					title={d.displayPath}
					onclick={() => revealDiff(d, key)}
					onmouseenter={() => setHoverHighlight(key)}
				>
					{#snippet extras()}
						{@render rowBadge(staged[d.key] ?? d)}
						{#if d.deployKind === 'raw_app'}
							<!-- Unloaded app leaf: row click reveals the diff, the chevron
							     loads the values and unwraps the file tree. -->
							<span
								role="button"
								tabindex="-1"
								title="Show files"
								class="w-5 h-5 flex items-center justify-center shrink-0 rounded cursor-pointer hover:bg-surface-secondary"
								onclick={(e) => {
									e.stopPropagation()
									expandApp(d)
								}}
							>
								{#if loadedDiffs[d.key]?.state === 'loading'}
									<Loader2 class="w-3 h-3 animate-spin text-tertiary" />
								{:else}
									<ChevronRight class="w-3 h-3 text-tertiary" />
								{/if}
							</span>
						{:else}
							<span class="w-5 shrink-0"></span>
						{/if}
					{/snippet}
				</WorkspaceItemRow>
			{/if}
		</div>
	{/if}
{/snippet}

<!-- Live (non-done) diff content only — the done state renders at the card
     level so the two can cross-animate on deploy. -->
{#snippet diffBlock(item: DeployItem)}
	{@const loaded = loadedDiffs[item.key]}
	{#if !mountedRows[item.key]}
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
			{#each rawAppItems(item, loaded) as sub (displayKey(sub))}
				<!-- Anchor target for the sidebar's per-file rows; scroll-mt clears the
				     app block's sticky header. -->
				<div
					id={`ws-diff-${displayKey(sub)}`}
					class="border-t border-light first:border-t-0 scroll-mt-10"
				>
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
			<div class="flex flex-row flex-1 min-h-0">
				{#if model.items.length > 0}
					<aside
						bind:this={sidebarRoot}
						onmousemove={() => (mouseActive = true)}
						class="flex-none w-96 flex flex-col min-h-0 pt-3 px-1"
					>
						<!-- Both containers reserve the same stable scrollbar gutter (the input
						     wrapper never actually scrolls), so input and tree rows share one
						     right edge. padRightForGutter tops the gutter up to the 6px inner
						     padding target so the visual right margin equals the left one no
						     matter the platform's scrollbar width (0 for overlay scrollbars). -->
						<div
							class="pl-2 pb-1 shrink-0 overflow-y-auto"
							style="scrollbar-gutter: stable;"
							use:padRightForGutter
						>
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
							class="flex-1 min-h-0 overflow-y-auto overflow-x-hidden pl-2 pt-1.5 pb-2 flex flex-col gap-1"
							style="scrollbar-gutter: stable;"
							use:padRightForGutter
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
				<main
					bind:this={mainScrollEl}
					bind:clientHeight={mainHeight}
					class="flex-1 min-w-0 overflow-y-auto bg-surface"
				>
					<div class="pl-1 pr-3 pt-3 pb-4 flex flex-col gap-3">
						{#if model.loading && model.items.length === 0}
							<div class="flex items-center gap-2 text-sm text-secondary py-8 self-center">
								<Loader2 class="w-4 h-4 animate-spin" />
								Loading changes...
							</div>
						{:else if model.error}
							<div class="text-sm text-red-600 dark:text-red-400 py-4">{model.error}</div>
						{:else if model.items.length === 0}
							<div class="text-sm text-secondary py-4">No changes.</div>
						{:else if orderedItems.length === 0}
							<div class="text-sm text-secondary py-4">No files match.</div>
						{:else}
							<div class="flex flex-col">
								{#each orderedItems as d (d.key)}
									<!-- While a deploy's success is being staged, the card renders its
									     pre-deploy snapshot so the state flip waits for the check-mark
									     beat instead of racing it. -->
									{@const view = staged[d.key] ?? d}
									{@const action = actionFor(view)}
									{@const editUrl = editUrlFor?.(d)}
									{@const status = model.statusOf(d.key)}
									<div
										id={rowId(d)}
										class="mb-6 scroll-mt-3 rounded-md border border-light transition-shadow duration-500 {flashKey ===
										d.key
											? 'ring-1 ring-border-accent'
											: 'ring-0 ring-transparent'}"
									>
										<div
											class="sticky top-0 z-30 bg-surface-tertiary rounded-t-md flex items-center gap-2 px-3 py-2 border-b border-transparent"
										>
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
												{@render rowBadge(view)}
												{#if status?.status === 'failed'}
													{@render deployFailed(d)}
												{:else if action.op !== 'none'}
													{#if action.secondary?.length}
														<Button
															variant="subtle"
															unifiedSize="sm"
															destructive
															disabled={model.deploying || !!staged[d.key]}
															onclick={() => confirmDiscard(d)}
														>
															{action.secondary[0].label}
														</Button>
													{/if}
													<div class="relative overflow-hidden rounded-md">
														<Button
															variant="accent"
															unifiedSize="sm"
															disabled={model.deploying ||
																!!staged[d.key] ||
																!model.deployPermission.ok}
															startIcon={status?.status === 'loading'
																? { icon: Loader2, classes: 'animate-spin' }
																: { icon: Save }}
															title={model.deployPermission.ok
																? undefined
																: model.deployPermission.reason}
															onclick={() => deployStaged(d)}
														>
															{action.label}
														</Button>
														{#if staged[d.key]}
															<!-- Success beat: green check flies in over the button
															     (SaveButton pattern), holds, then the row flips. -->
															<div
																class="absolute inset-0 flex items-center justify-center bg-green-200 dark:bg-green-800 rounded-md"
																transition:fly={{ y: -10, duration: 300 }}
															>
																<CheckCircle2 class="w-4 h-4 text-green-700 dark:text-green-300" />
															</div>
														{/if}
													</div>
												{/if}
											</div>
										</div>
										<div class="bg-surface-tertiary rounded-b-md overflow-hidden">
											<!-- Deploy collapses the (tall) diff into the one-line done note —
											     slide both branches so the card height animates instead of
											     snapping. Transitions are local: nothing plays on drawer open. -->
											{#if view.done}
												<div
													transition:slide={{ duration: 300, easing: cubicOut }}
													class="text-2xs text-tertiary p-3"
												>
													Deployed — no pending changes.
												</div>
											{:else}
												<div transition:slide={{ duration: 300, easing: cubicOut }}>
													{@render diffBlock(view)}
												</div>
											{/if}
										</div>
									</div>
								{/each}
							</div>
							<!-- Just enough trailing room for the last item to reach the top;
							     without it the container bottoms out and short lists sit low. -->
							<div aria-hidden="true" class="shrink-0" style="height: {trailingSpace}px"></div>
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
			<!-- Inside the drawer subtree: the drawer is portaled/elevated, so a
			     sibling modal would stack beneath it. -->
			<ConfirmationModal {...discardConfirm.props} />
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
