<script lang="ts">
	import TreeView from './TreeView.svelte'
	import { onDestroy, untrack } from 'svelte'
	import ResizeTransitionWrapper from '$lib/components/common/ResizeTransitionWrapper.svelte'

	import { ChevronDown, ChevronUp, Folder, FolderTree, NetworkIcon, User } from 'lucide-svelte'
	import Item from './Item.svelte'
	import { countLeaves, type FolderItem, type ItemType, type UserItem } from './treeViewUtils'
	import { twMerge } from 'tailwind-merge'
	import { pluralize } from '$lib/utils'
	import { base } from '$lib/base'
	import { Button } from '$lib/components/common'

	interface Props {
		item: ItemType | FolderItem | UserItem
		collapseAll: boolean
		depth?: number
		showCode: (path: string, summary: string) => void
		isSearching?: boolean
		pipelineFolders?: Set<string>
		// How many runnables each `f/<folder>` / `u/<user>` holds for this user, keyed by
		// full prefix. Known before an owner is expanded, unlike its loaded rows.
		ownerCounts?: Record<string, number>
		// Lazy loading state, keyed by the full path prefix a node covers: `f/<name>` /
		// `u/<name>` for a top-level owner, `<parent>/<subfolder>` deeper. A node paginates
		// within its own prefix, so a subfolder can be completed without paging everything
		// its owner holds.
		ownerLoad?: Record<
			string,
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean }
		>
		// `all` pages the prefix to the end in one call instead of fetching a single page.
		onExpandOwner?: (prefix: string, more?: boolean, opts?: { all?: boolean }) => void
		onCollapseOwner?: (prefix: string) => void
		// Position of this node among the rendered root nodes; "expand all" only
		// auto-loads the first EXPAND_ALL_LOAD_LIMIT of them (see the effect below).
		rootIndex?: number
		showEditButton?: boolean
		// Path prefix of the parent node, so this one can name its own (`ownerLoad` and
		// the listing endpoint are both keyed by full prefix). Unset at the top level.
		parentPrefix?: string
		// The nearest ancestor that was loaded directly still has unloaded pages, so what
		// is grouped under this node is only part of it: counts render as "N+" and the
		// node offers to load the rest of itself.
		ancestorHasMore?: boolean
	}

	let {
		item,
		collapseAll,
		depth = 0,
		showCode,
		isSearching = false,
		pipelineFolders,
		ownerCounts,
		ownerLoad,
		onExpandOwner,
		onCollapseOwner,
		rootIndex = 0,
		showEditButton = true,
		parentPrefix,
		ancestorHasMore = false
	}: Props = $props()

	// Bounds the request burst from "expand all": however many root owners the tree
	// renders (its slice grows as you scroll), it fetches at most this many. Lazy owners
	// past the cap stay collapsed and load on a single click (see the effect).
	const EXPAND_ALL_LOAD_LIMIT = 20

	const isFolderItem = (i: typeof item): i is FolderItem => i && 'folderName' in i
	const isFolder = isFolderItem
	const isUser = (i: typeof item): i is UserItem => i && 'username' in i

	// Hidden while searching: pipelines aren't part of the text filter (the list
	// view hides their rows on a query too), so a folder matching the search
	// shouldn't surface an unrelated Pipeline row.
	let hasPipeline = $derived(
		depth === 0 &&
			!isSearching &&
			isFolderItem(item) &&
			(pipelineFolders?.has(item.folderName) ?? false)
	)

	// Starts closed and is opened by the collapseAll effect below, which runs after the
	// first render. Starting open instead would mount this node's whole loaded subtree for
	// that one frame — thousands of rows once a few pages are in, since a node in lazy mode
	// renders every loaded row — every time an ancestor opens or remounts.
	let opened: boolean = $state(false)

	// Full path prefix of this node when it's a top-level owner (folder or user).
	let ownerKey = $derived(
		depth === 0
			? isFolder(item)
				? `f/${item.folderName}`
				: isUser(item)
					? `u/${item.username}`
					: undefined
			: undefined
	)
	// Full path prefix this node covers, at any depth: the owner at the top level, then
	// the parent's prefix plus this folder's segment. It is what both `ownerLoad` and the
	// listing endpoint's `pathStart` are keyed by.
	let nodePrefix = $derived(
		depth === 0
			? ownerKey
			: parentPrefix != undefined && isFolder(item)
				? `${parentPrefix}/${item.folderName}`
				: undefined
	)
	// A top-level owner in lazy mode: its items load on demand into a separate store
	// (ownerLoad tracks per-prefix state), so its count/pagination differ from a node
	// whose items are already grouped from the loaded window.
	let isLazyOwner = $derived(ownerKey != undefined && ownerLoad != undefined)
	let ownerState = $derived(ownerKey != undefined ? ownerLoad?.[ownerKey] : undefined)
	// This node's own load state — set once it has been loaded directly (an owner on
	// expand, a subfolder only when its "Load more" was used).
	let nodeState = $derived(nodePrefix != undefined ? ownerLoad?.[nodePrefix] : undefined)
	// Whether rows under this node are still incomplete: its own pagination once it has
	// been loaded directly, otherwise whatever its nearest loaded ancestor reports.
	let nodeHasMore = $derived(nodeState?.loaded ? nodeState.hasMore : ancestorHasMore)
	// Rows loaded under this node. Its children count a subfolder as one entry, which
	// would label a folder holding 133 rows in one subfolder as "1 item"; every count this
	// node shows (badge and pager alike) means leaves, so they can't contradict.
	let loadedHere = $derived(isFolder(item) || isUser(item) ? countLeaves(item) : 0)
	// The owner's runnable count alone (no pipeline row), so it lines up with the rows
	// actually fetched for it — what the partial-load footer compares against. An owner
	// missing from `ownerCounts` holds nothing (the response omits those), so it reads as
	// 0 rather than unknown. Floored by what is already rendered under the node: the count
	// can come in under that (it can miss an item shared individually out of a folder the
	// viewer isn't in), and a total below the rows beneath it reads as a bug. The owner
	// chips floor the same way.
	let ownerTotal = $derived(
		ownerKey != undefined && ownerCounts != undefined
			? Math.max(ownerCounts[ownerKey] ?? 0, loadedHere)
			: undefined
	)
	// What this owner renders, known without expanding it. Preferred over the loaded
	// rows, which are one page deep and count a subfolder as a single child. The pipeline
	// entry is a row of its own and its member scripts are excluded from the count, so it
	// adds one where it renders.
	let ownerCount = $derived(
		ownerTotal != undefined ? ownerTotal + (hasPipeline ? 1 : 0) : undefined
	)

	// How many children a node renders before "Show more". Kept well above a screenful
	// so a subfolder's contents don't look truncated, but bounded: under "expand all"
	// this applies to every open owner at once (see effectiveMax).
	let showMax = $state(30)
	// Ceiling on what one node mounts at once. Comfortably past a server page, so the
	// usual node still shows everything it loaded, but "Load all" can leave thousands of
	// rows under one prefix and mounting all of them locks up the tab.
	const LAZY_RENDER_MAX = 500
	// A node that paginates server-side ("Load more") shows all its already-loaded rows
	// when opened on its own, so there is one control to reach the rest and not a second,
	// confusing client "Show more" in front of it. EXCEPT under "expand all"
	// (collapseAll=false), which opens every visible node at once — rendering all of each
	// would be thousands of rows and freeze the tab — so there we cap to the client slice
	// and let "Show more" reveal the rest per node.
	let effectiveMax = $derived(
		isFolder(item) || isUser(item)
			? ownerLoad != undefined && nodePrefix != undefined && collapseAll
				? Math.min(item.items.length, Math.max(showMax, LAZY_RENDER_MAX))
				: Math.min(item.items.length, showMax)
			: showMax
	)
	// Which of the two footer buttons started the run in flight, so only that one spins: a
	// "Load all" can take minutes where a "Load more" takes one request.
	let loadingAll = $state(false)
	$effect(() => {
		if (!nodeState?.loading) loadingAll = false
	})
	// One "Show more" reveals a slice the size of the ceiling once a node holds more than
	// that, so thousands of loaded rows don't take hundreds of clicks to unfold. Keyed off
	// what the node holds rather than what it renders: under "expand all" only the small
	// client slice is on screen however many rows arrived.
	let showMoreStep = $derived(
		(isFolder(item) || isUser(item)) && item.items.length >= LAZY_RENDER_MAX ? LAZY_RENDER_MAX : 30
	)

	$effect(() => {
		const expandAll = !collapseAll
		untrack(() => {
			// "Expand all" opens every node — EXCEPT LAZY top-level owners past the request
			// cap: opening those without loading would leave them empty and take two clicks
			// to load (the first would just collapse). Keep them closed so one click
			// opens+loads them. Only lazy owners (ownerLoad present) issue requests, so a
			// non-lazy owner (e.g. label-filtered mode, already loaded) is never capped.
			const cappedOwner =
				expandAll &&
				depth === 0 &&
				ownerKey != undefined &&
				ownerLoad != undefined &&
				rootIndex >= EXPAND_ALL_LOAD_LIMIT
			const shouldOpen = expandAll && !cappedOwner
			opened = shouldOpen
			if (ownerKey == undefined) return
			if (shouldOpen) {
				// Open+load this owner (onExpandOwner is a no-op for non-lazy owners).
				onExpandOwner?.(ownerKey)
			} else {
				// Closed here (collapse all, or a capped lazy owner): untrack it so a later
				// reload doesn't re-fetch a hidden owner, recreating the fan-out openOwners
				// exists to prevent.
				onCollapseOwner?.(ownerKey)
			}
		})
	})
	// A node can go away without a collapse click — its owner dropped once the counts
	// show it empty, or sliced out of the rendered window. Untracking it keeps the
	// reload fan-out tied to what is actually on screen; without this a dropped owner
	// stays registered and every later reload re-fetches it forever. Remounting
	// re-registers it (see the collapseAll effect above).
	onDestroy(() => {
		// nodePrefix, not ownerKey: a subfolder that was loaded directly is tracked under
		// its own prefix and has to be untracked the same way.
		const key = untrack(() => nodePrefix)
		if (key != undefined) onCollapseOwner?.(key)
	})

	let lastToggle = 0
	function toggleOwner() {
		// A double-click would otherwise toggle twice — expand then immediately collapse,
		// and for a lazy owner waste the fetch the first click kicked off, which reads as
		// the node "expanding and collapsing at once". Swallow a second toggle landing
		// within 300ms so a rapid double-click settles open.
		const now = Date.now()
		if (now - lastToggle < 300) return
		lastToggle = now
		opened = !opened
		if (opened) {
			// A top-level owner loads on expand. A subfolder's rows come from an ancestor's
			// pages, so opening one must not fire a request per subfolder — its own "Load
			// more" is the explicit way to complete it. One exception, and it costs nothing:
			// a subfolder that has paged itself — `nodeState` exists once it has, including
			// while that first load is still in flight — re-registers so later reloads keep
			// refreshing the rows it is showing, instead of letting a parent refresh silently
			// truncate them. loadOwnerItems starts no second request for either state.
			if (nodePrefix != undefined && (ownerKey != undefined || nodeState != undefined))
				onExpandOwner?.(nodePrefix)
		} else if (nodePrefix != undefined) {
			// Any depth: a closed node stays mounted, so without this a subfolder that
			// had paged itself would keep being re-fetched by every later reload while
			// none of its rows are on screen.
			onCollapseOwner?.(nodePrefix)
		}
	}
</script>

{#if isFolder(item) || isUser(item)}
	<div>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onclick={toggleOwner}
			class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
		>
			<div
				class={twMerge('flex flex-row items-center gap-4 text-sm font-semibold')}
				style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
			>
				<div class="flex justify-center items-center">
					{#if isUser(item)}
						<User size={16} class="text-secondary" />
					{:else if depth === 0}
						<Folder size={16} class="text-secondary" />
					{:else}
						<FolderTree size={16} class="text-secondary" />
					{/if}
				</div>

				<div>
					<span class="whitespace-nowrap text-xs text-emphasis font-semibold">
						{#if isUser(item)}u/{item.username}{:else}{#if depth === 0}f/{/if}{item.folderName}{/if}
					</span>
					<div class="text-2xs font-normal text-secondary whitespace-nowrap">
						{#if ownerCount != undefined}
							({pluralize(ownerCount, ' item')})
						{:else if isLazyOwner && !ownerState?.loaded}
							<!-- Lazy owner not expanded yet and no count for it: its true item count
							     is unknown until loaded, so showing "(0 items)" would be misleading. -->
							&nbsp;
						{:else if nodeHasMore}
							<!-- Partial: this node still has pages to load (its own once it has been
							     loaded directly, otherwise its ancestor's), so what is grouped under
							     it is only what has arrived so far. -->
							({loadedHere}+ items)
						{:else}
							({pluralize(loadedHere, ' item')})
						{/if}
					</div>
				</div>
			</div>
			<button class="w-full flex flex-row-reverse">
				{#if opened}
					<ChevronUp size={16} />
				{:else}
					<ChevronDown size={16} />
				{/if}
			</button>
		</div>
		<!-- ResizeObserver, not a slide: a freshly-opened owner fetches its rows, so its height
		     changes twice (open-empty, then rows land) and a slide would only animate the first. -->
		<ResizeTransitionWrapper vertical innerClass="w-full">
			{#if opened || isSearching}
				<div>
					{#if hasPipeline && isFolder(item)}
						<!-- py-3 matches common/table/Row.svelte so this row sits at
					     the same height as the script/flow/app rows that follow
					     it under the same folder; py-2 was visibly shorter. -->
						<a
							href="{base}/pipeline/{encodeURIComponent(item.folderName)}"
							class="flex items-center gap-4 px-4 py-3 border-b text-sm hover:bg-surface-hover transition-colors"
							style="padding-left: {(depth + 1) * 16}px;"
						>
							<NetworkIcon size={16} class="text-emerald-600 dark:text-emerald-400" />
							<span class="text-xs font-medium text-emphasis">Pipeline</span>
						</a>
					{/if}
					{#each item.items.slice(0, effectiveMax) as subItem, index ((subItem['path'] ? subItem['type'] + '__' + subItem['path'] + '__' + index : undefined) ?? 'folder__' + subItem['folderName'] + '__' + index)}
						<TreeView
							{isSearching}
							{collapseAll}
							item={subItem}
							{pipelineFolders}
							{ownerLoad}
							{onExpandOwner}
							{onCollapseOwner}
							parentPrefix={nodePrefix}
							ancestorHasMore={nodeHasMore}
							on:scriptChanged
							on:flowChanged
							on:appChanged
							on:rawAppChanged
							on:reload
							{showCode}
							{showEditButton}
							depth={depth + 1}
						/>
					{/each}
					{#if effectiveMax < item.items.length}
						<div
							class="px-4 py-2 border-b flex flex-row items-center justify-between gap-4 bg-surface-secondary"
							style="padding-left: {(depth + 1) * 16}px;"
						>
							<!-- Rows, not items: this slices the node's own entries, where a subfolder
						     is one row standing for everything under it. -->
							<span class="text-xs text-secondary">
								Showing {effectiveMax} of {item.items.length} loaded rows
							</span>
							<Button
								unifiedSize="sm"
								variant="subtle"
								on:click={() => {
									// Grown from what is rendered, not from showMax: the lazy ceiling can
									// already be showing more than showMax, and stepping that would take
									// several clicks to change anything on screen.
									showMax = Math.min(item.items.length, effectiveMax + showMoreStep)
								}}
							>
								Show more
							</Button>
						</div>
					{/if}
					{#if nodePrefix != undefined && ownerLoad != undefined}
						{#if nodeState?.loading && item.items.length === 0}
							<!-- Show the spinner only on the first load, when there's nothing yet. A
						     re-sort/re-filter re-fetch keeps the old rows visible and swaps them
						     in place, so flashing "Loading…" under them would just be noise. -->
							<div class="text-center text-xs py-2 text-secondary">Loading…</div>
						{:else if nodeHasMore && (collapseAll || nodeState?.loading || effectiveMax >= item.items.length)}
							<!-- Every folder pages within its own prefix, so completing a subfolder
						     doesn't mean paging everything its owner holds. Under "expand all" this
						     waits for the client "Show more" above, so the two pagers don't stack
						     under every open node at once — but never while loading, or a long run
						     would unmount its own spinner on its first page. Spelling out the counts
						     is the point: without them this reads as an optional extra rather than
						     as rows still missing. -->
							<div
								class="px-4 py-2 border-b flex flex-row items-center justify-between gap-4 bg-surface-secondary"
								style="padding-left: {(depth + 1) * 16}px;"
							>
								<span class="text-xs text-secondary">
									Showing {loadedHere}{ownerTotal != undefined ? ` of ${ownerTotal}` : ''} items in {nodePrefix}
								</span>
								<div class="flex flex-row items-center gap-2 shrink-0">
									<Button
										unifiedSize="sm"
										variant="subtle"
										loading={nodeState?.loading && !loadingAll}
										disabled={nodeState?.loading}
										on:click={() =>
											nodePrefix != undefined &&
											onExpandOwner?.(nodePrefix, nodeState?.loaded ?? false)}
									>
										Load more
									</Button>
									<!-- Same call, paged to the end: a folder several pages deep otherwise
								     takes a click per page to reach an exact count. -->
									<Button
										unifiedSize="sm"
										variant="subtle"
										loading={nodeState?.loading && loadingAll}
										disabled={nodeState?.loading}
										on:click={() => {
											if (nodePrefix == undefined) return
											loadingAll = true
											onExpandOwner?.(nodePrefix, nodeState?.loaded ?? false, { all: true })
										}}
									>
										Load all
									</Button>
								</div>
							</div>
						{/if}
					{/if}
				</div>
			{/if}
		</ResizeTransitionWrapper>
	</div>
{:else}
	<Item
		{item}
		{showCode}
		{showEditButton}
		on:scriptChanged
		on:flowChanged
		on:appChanged
		on:rawAppChanged
		on:reload
		{depth}
	/>
{/if}
