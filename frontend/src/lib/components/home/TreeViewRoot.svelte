<script lang="ts">
	import { untrack } from 'svelte'
	import TreeView from './TreeView.svelte'
	import { countLeaves, groupItems, type ItemType, type UserItem } from './treeViewUtils'
	import { Button } from '$lib/components/common'
	import { ChevronDown, ChevronUp, Users } from 'lucide-svelte'
	import { getLocalSetting, pluralize, storeLocalSetting } from '$lib/utils'

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
		// Tuck every user space but `selfUsername`'s into one collapsible "Other users" row
		// under it. Only for browsing: a search or filter must not hide its matches behind
		// a closed row.
		groupOtherUsers?: boolean
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
		showEditButton = true,
		groupOtherUsers = false
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

	type RootNode = ReturnType<typeof groupItems>[number]
	// `loadRank` is what "expand all" caps its requests by (see TreeView's rootIndex).
	type RootRow = { kind: 'node'; node: RootNode; loadRank: number } | { kind: 'otherUsers' }

	const OTHER_USERS_OPEN_SETTING_NAME = 'homeTreeOtherUsersOpen'
	let otherUsersOpen = $state(getLocalSetting(OTHER_USERS_OPEN_SETTING_NAME) == 'true')
	function toggleOtherUsers() {
		otherUsersOpen = !otherUsersOpen
		storeLocalSetting(OTHER_USERS_OPEN_SETTING_NAME, otherUsersOpen ? 'true' : undefined)
	}
	// The group pages its rows separately from the root slice. Drawing from `nbDisplayed`
	// instead, opening it would push folders past the slice and unmount them, dropping
	// whatever they had expanded.
	let nbOtherUsersDisplayed = $state(ROOT_PAGE)

	function isOtherUser(node: RootNode): node is UserItem {
		return 'username' in node && node.username !== selfUsername
	}

	let otherUsers: UserItem[] = $derived(
		groupOtherUsers && selfUsername != undefined && Array.isArray(groupedItems)
			? groupedItems.filter(isOtherUser)
			: []
	)
	let otherUsersItemCount = $derived(
		ownerCounts != undefined
			? otherUsers.reduce(
					(sum, u) => sum + Math.max(ownerCounts[`u/${u.username}`] ?? 0, countLeaves(u)),
					0
				)
			: undefined
	)

	let rows: RootRow[] = $derived.by(() => {
		if (!Array.isArray(groupedItems)) return []
		if (otherUsers.length === 0) {
			return groupedItems.map((node, i): RootRow => ({ kind: 'node', node, loadRank: i }))
		}
		// `groupItems` orders users before folders, so with the others taken out the viewer's
		// own space is what leads.
		const main = groupedItems.filter((g) => !isOtherUser(g))
		const lead = main.filter((g) => 'username' in g)
		const rest = main.filter((g) => !('username' in g))
		return [
			...lead.map((node, i): RootRow => ({ kind: 'node', node, loadRank: i })),
			{ kind: 'otherUsers' },
			...rest.map((node, i): RootRow => ({ kind: 'node', node, loadRank: lead.length + i }))
		]
	})
	// Owner rows only, for the footer: the "Other users" row is neither a folder nor a user.
	let ownerRowCount = $derived(rows.filter((r) => r.kind === 'node').length)
	let shownOwnerRowCount = $derived(
		rows.slice(0, nbDisplayed).filter((r) => r.kind === 'node').length
	)

	let footerEl: HTMLDivElement | undefined = $state()
	// Reveal the next slice of root nodes as the footer comes into view. Only the
	// client-side slice auto-grows — those nodes are already grouped and render
	// collapsed, so this issues no request; paging the server stays behind the button.
	$effect(() => {
		const el = footerEl
		if (!el) return
		const observer = new IntersectionObserver((entries) => {
			if (!entries.some((e) => e.isIntersecting)) return
			if (nbDisplayed >= rows.length) return
			if (nbDisplayed >= AUTO_REVEAL_LIMIT) return
			nbDisplayed = Math.min(nbDisplayed + ROOT_PAGE, rows.length)
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

{#snippet ownerNode(node: RootNode, loadRank: number, indent: number)}
	<TreeView
		rootIndex={loadRank}
		{indent}
		{isSearching}
		{collapseAll}
		item={node}
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
{/snippet}

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
		{#each rows.slice(0, nbDisplayed) as row (row.kind === 'otherUsers' ? 'other_users' : 'folderName' in row.node ? `f__${row.node.folderName}` : 'username' in row.node ? `u__${row.node.username}` : `i__${row.node.type}__${row.node.path}`)}
			{#if row.kind === 'otherUsers'}
				<!-- Same shape as an owner row, so it reads as part of the tree. It only reveals
				     the user rows under it: each still loads its own items when opened. -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					onclick={toggleOtherUsers}
					class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
				>
					<div class="flex flex-row items-center gap-4">
						<Users size={16} class="text-secondary" />
						<div>
							<span class="whitespace-nowrap text-xs text-emphasis font-semibold">Other users</span>
							<div class="text-2xs font-normal text-secondary whitespace-nowrap">
								({pluralize(otherUsers.length, 'user')}{otherUsersItemCount != undefined
									? ` · ${pluralize(otherUsersItemCount, 'item')}`
									: ''})
							</div>
						</div>
					</div>
					<Button
						iconOnly
						unifiedSize="xs"
						variant="subtle"
						startIcon={{ icon: otherUsersOpen ? ChevronUp : ChevronDown }}
						title={otherUsersOpen ? 'Hide other users' : 'Show other users'}
						aria-label="Other users"
						aria-expanded={otherUsersOpen}
						onClick={toggleOtherUsers}
					/>
				</div>
				{#if otherUsersOpen}
					<!-- Ranked after every root owner, so opening this row can't push folders out
					     of what "expand all" auto-loads. -->
					{#each otherUsers.slice(0, nbOtherUsersDisplayed) as user, i (user.username)}
						{@render ownerNode(user, ownerRowCount + i, 1)}
					{/each}
					{#if nbOtherUsersDisplayed < otherUsers.length}
						<div
							class="pl-8 pr-4 py-2 border-b flex flex-row items-center justify-between gap-4 bg-surface-secondary"
						>
							<span class="text-xs text-secondary">
								Showing {nbOtherUsersDisplayed} of {otherUsers.length} users
							</span>
							<Button
								unifiedSize="sm"
								variant="subtle"
								onClick={() =>
									(nbOtherUsersDisplayed = Math.min(
										nbOtherUsersDisplayed + ROOT_PAGE,
										otherUsers.length
									))}
							>
								Show more
							</Button>
						</div>
					{/if}
				{/if}
			{:else}
				{@render ownerNode(row.node, row.loadRank, 0)}
			{/if}
		{/each}
		{#if nbDisplayed < rows.length || hasMoreServer}
			<!-- Last row of the tree's own frame, not a caption under it: what is missing
			     has to read as part of the list to be noticed at all. -->
			<div
				bind:this={footerEl}
				class="px-4 py-3 flex flex-row items-center justify-between gap-4 bg-surface-secondary"
			>
				<span class="text-xs text-secondary">
					{#if nbDisplayed < rows.length}
						Showing {shownOwnerRowCount} of {ownerRowCount} folders and users
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
						if (nbDisplayed < rows.length)
							nbDisplayed = Math.min(nbDisplayed + ROOT_PAGE, rows.length)
						else onLoadMore?.()
					}}
				>
					{nbDisplayed < rows.length ? 'Show more' : 'Load more'}
				</Button>
			</div>
		{/if}
	</div>
{/if}
