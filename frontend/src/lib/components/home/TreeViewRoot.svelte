<script lang="ts">
	import { untrack } from 'svelte'
	import TreeView from './TreeView.svelte'
	import { groupItems, type ItemType } from './treeViewUtils'
	import { Button } from '$lib/components/common'

	interface Props {
		collapseAll: boolean
		showCode: (path: string, summary: string) => void
		items: ItemType[] | undefined
		isSearching?: boolean
		pipelineFolders?: Set<string>
		sortCompare?: (a: ItemType, b: ItemType) => number
		// Order of the top-level folder/user nodes: Z-A when the active sort is
		// name-descending (like a file explorer), alphabetical otherwise.
		groupDesc?: boolean
		// The server has further pages beyond the loaded items; `onLoadMore` fetches
		// the next one (grouping only reorders what's already loaded).
		hasMoreServer?: boolean
		onLoadMore?: () => void
		// Lazy per-owner loading: every folder and every user shows as a top-level node
		// regardless of the loaded window; expanding one loads its items on demand,
		// paginated within it. `ownerLoad` keys are full path prefixes (`f/<name>` /
		// `u/<name>`).
		allFolders?: string[]
		allUsers?: string[]
		// How many runnables each `f/<folder>` / `u/<user>` holds for this user, keyed
		// by full prefix; owners holding none are absent. Undefined while it loads or
		// when it doesn't apply, in which case every owner is injected as before.
		ownerCounts?: Record<string, number>
		// The viewer's own username: their personal space is a fixture of the tree and
		// stays even when empty, so they have somewhere to create into.
		selfUsername?: string
		ownerLoad?: Record<
			string,
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean }
		>
		onExpandOwner?: (owner: string, more?: boolean, opts?: { all?: boolean }) => void
		onCollapseOwner?: (owner: string) => void
		showEditButton?: boolean
	}

	let {
		collapseAll,
		showCode,
		items,
		isSearching = false,
		pipelineFolders,
		sortCompare,
		groupDesc = false,
		hasMoreServer = false,
		onLoadMore,
		allFolders = [],
		allUsers = [],
		ownerCounts,
		selfUsername,
		ownerLoad,
		onExpandOwner,
		onCollapseOwner,
		showEditButton = true
	}: Props = $props()

	// How many root nodes render at once. A root node is a collapsed owner row that
	// fetches nothing until expanded, so a large slice costs a row each and no request
	// — and an owner sliced off the end is indistinguishable from one that doesn't
	// exist, so keep it well above the number of folders a workspace typically has.
	const ROOT_PAGE = 100
	// Ceiling on what scrolling alone reveals: root rows aren't virtualized, so on a
	// workspace with thousands of owners one long scroll gesture would otherwise mount
	// every one of them. Past this the footer stays put and its button reveals the rest.
	const AUTO_REVEAL_LIMIT = 500
	let nbDisplayed = $state(ROOT_PAGE)

	let groupedItems: ReturnType<typeof groupItems> | 'loading' = $state('loading')
	$effect(() => {
		items
		pipelineFolders
		isSearching
		sortCompare
		groupDesc
		allFolders
		allUsers
		ownerCounts
		selfUsername
		untrack(() => {
			// While searching, `items` is already relevance-ranked and the sort
			// selector is disabled, so keep that order: a no-op leaf comparator
			// preserves insertion order within each group (Array.sort is stable).
			const grouped = groupItems(items, isSearching ? () => 0 : sortCompare, groupDesc)
			// Ensure every pipeline folder is present at the top level so its
			// "Pipeline" entry shows even when it has no listed items — a bundle-phase
			// pipeline (only a draft so far) or a folder whose only scripts are
			// pipeline members (folded into the pipeline, hidden from the list).
			// Skip while searching: pipelines aren't part of the text filter (list view
			// hides them on `filter !== ''`), so injecting them would surface unrelated
			// folders in the results.
			if (!isSearching) {
				// Inject a top-level node for every pipeline folder (so its Pipeline entry
				// shows even with no listed items), every workspace folder, and every user
				// — so an owner whose items sit outside the loaded window still appears;
				// expanding one loads its items on demand (see onExpandOwner). Injecting
				// users too is what stops a user node from vanishing under a name sort whose
				// first page is all folder rows.
				const presentFolders = new Set(
					grouped
						.filter((g) => 'folderName' in g)
						.map((g) => (g as { folderName: string }).folderName)
				)
				// Once counts are in, an owner holding nothing the user can see is dropped
				// rather than injected — a workspace's folder list is mostly noise in the
				// tree otherwise. Pipeline folders are exempt: their entry is the pipeline
				// itself, whose member scripts are folded out of the runnable count.
				const isEmptyOwner = (prefix: string) => ownerCounts != undefined && !ownerCounts[prefix]
				const missingFolders: { folderName: string; items: [] }[] = []
				for (const folderName of [...(pipelineFolders ?? []), ...allFolders]) {
					if (presentFolders.has(folderName)) continue
					if (!pipelineFolders?.has(folderName) && isEmptyOwner(`f/${folderName}`)) continue
					presentFolders.add(folderName)
					missingFolders.push({ folderName, items: [] })
				}
				const presentUsers = new Set(
					grouped.filter((g) => 'username' in g).map((g) => (g as { username: string }).username)
				)
				const missingUsers: { username: string; items: [] }[] = []
				for (const username of allUsers) {
					if (presentUsers.has(username)) continue
					// Your own space is exempt from the drop: it's where you create, so it
					// stays visible (as "0 items") when empty rather than disappearing.
					if (username !== selfUsername && isEmptyOwner(`u/${username}`)) continue
					presentUsers.add(username)
					missingUsers.push({ username, items: [] })
				}
				if (missingFolders.length || missingUsers.length) {
					// `groupItems` returns user groups first, then folders alphabetically.
					// Append the missing nodes and sort each section once (O(n log n)) rather
					// than splicing each in with findIndex (O(n²) — at 10k owners that was
					// ~50M comparisons on every page merge).
					const dir = groupDesc ? -1 : 1
					const users = grouped.filter((g) => 'username' in g) as { username: string }[]
					const folders = grouped.filter((g) => 'folderName' in g) as { folderName: string }[]
					users.push(...missingUsers)
					folders.push(...missingFolders)
					users.sort((a, b) => dir * a.username.localeCompare(b.username))
					folders.sort((a, b) => dir * a.folderName.localeCompare(b.folderName))
					grouped.length = 0
					grouped.push(
						...(users as unknown as typeof grouped),
						...(folders as unknown as typeof grouped)
					)
				}
			}
			groupedItems = grouped
		})
	})

	let footerEl: HTMLDivElement | undefined = $state()
	// Reveal the next slice of root nodes as the footer comes into view. Only the
	// client-side slice auto-grows — those nodes are already grouped and render
	// collapsed, so this issues no request; paging the server stays behind the button.
	$effect(() => {
		const el = footerEl
		if (!el) return
		const observer = new IntersectionObserver((entries) => {
			if (!entries.some((e) => e.isIntersecting)) return
			const grouped = groupedItems
			if (!Array.isArray(grouped) || nbDisplayed >= grouped.length) return
			if (nbDisplayed >= AUTO_REVEAL_LIMIT) return
			nbDisplayed = Math.min(nbDisplayed + ROOT_PAGE, grouped.length)
			// Revealing more doesn't change whether the footer intersects, so no further
			// callback would fire and scrolling would stall with rows left unrevealed.
			// Re-observing re-delivers the current intersection after the rows render.
			observer.unobserve(el)
			observer.observe(el)
		})
		observer.observe(el)
		return () => observer.disconnect()
	})
</script>

{#if groupedItems === 'loading'}
	<div class="flex flex-row items-center justify-center">
		<div class="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 dark:border-gray-100"
		></div>
	</div>
{:else if groupedItems.length === 0}
	<div class="flex flex-row items-center justify-center">
		<div class="text-xs font-normal text-hint">No items</div>
	</div>
{:else}
	<div class="border rounded-md bg-surface-tertiary">
		{#each groupedItems.slice(0, nbDisplayed) as item, rootIndex ('folderName' in item ? `f__${item.folderName}` : 'username' in item ? `u__${item.username}` : `i__${item.type}__${item.path}`)}
			{#if item}
				<TreeView
					{rootIndex}
					{isSearching}
					{collapseAll}
					{item}
					{pipelineFolders}
					{ownerCounts}
					ancestorHasMore={hasMoreServer}
					{ownerLoad}
					{onExpandOwner}
					{onCollapseOwner}
					on:scriptChanged
					on:flowChanged
					on:appChanged
					on:rawAppChanged
					on:reload
					{showCode}
					{showEditButton}
				/>
			{/if}
		{/each}
		{#if nbDisplayed < groupedItems.length || hasMoreServer}
			<!-- Last row of the tree's own frame, not a caption under it: what is missing
			     has to read as part of the list to be noticed at all. -->
			<div
				bind:this={footerEl}
				class="px-4 py-3 flex flex-row items-center justify-between gap-4 bg-surface-secondary"
			>
				<span class="text-xs text-secondary">
					{#if nbDisplayed < groupedItems.length}
						Showing {nbDisplayed} of {groupedItems.length} folders and users
					{:else}
						<!-- Scoped to one owner: the tree groups the paged browse stream, so what
						     is missing is items, not root nodes. -->
						Not all items are loaded yet
					{/if}
				</span>
				<Button
					unifiedSize="sm"
					variant="subtle"
					on:click={() => {
						if (nbDisplayed < groupedItems.length)
							nbDisplayed = Math.min(nbDisplayed + ROOT_PAGE, groupedItems.length)
						else onLoadMore?.()
					}}
				>
					{nbDisplayed < groupedItems.length ? 'Show more' : 'Load more'}
				</Button>
			</div>
		{/if}
	</div>
{/if}
