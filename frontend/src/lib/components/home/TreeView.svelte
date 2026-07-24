<script lang="ts">
	import TreeView from './TreeView.svelte'
	import { untrack } from 'svelte'

	import { ChevronDown, ChevronUp, Folder, FolderTree, NetworkIcon, User } from 'lucide-svelte'
	import Item from './Item.svelte'
	import type { FolderItem, ItemType, UserItem } from './treeViewUtils'
	import { twMerge } from 'tailwind-merge'
	import { pluralize } from '$lib/utils'
	import { base } from '$lib/base'

	interface Props {
		item: ItemType | FolderItem | UserItem
		collapseAll: boolean
		depth?: number
		showCode: (path: string, summary: string) => void
		isSearching?: boolean
		pipelineFolders?: Set<string>
		// Lazy per-owner loading (top-level folders and users only): expanding an owner
		// loads its items on demand and paginates within it. `ownerLoad` keys are full
		// path prefixes (`f/<name>` / `u/<name>`).
		ownerLoad?: Record<
			string,
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean; count: number }
		>
		onExpandOwner?: (owner: string, more?: boolean) => void
		onCollapseOwner?: (owner: string) => void
		// Position of this node among the rendered root nodes; "expand all" only
		// auto-loads the first EXPAND_ALL_LOAD_LIMIT of them (see the effect below).
		rootIndex?: number
	}

	let {
		item,
		collapseAll,
		depth = 0,
		showCode,
		isSearching = false,
		pipelineFolders,
		ownerLoad,
		onExpandOwner,
		onCollapseOwner,
		rootIndex = 0
	}: Props = $props()

	// Bounds the request burst from "expand all": however many root owners are rendered
	// (nbDisplayed can grow via "load 30 more"), it fetches at most this many. Owners
	// past the cap open empty and load on an explicit click.
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

	let opened: boolean = $state(true)

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
	// A top-level owner in lazy mode: its items load on demand into a separate store
	// (ownerLoad tracks per-owner state), so its count/pagination differ from a node
	// whose items are already grouped from the loaded window.
	let isLazyOwner = $derived(ownerKey != undefined && ownerLoad != undefined)
	let ownerState = $derived(ownerKey != undefined ? ownerLoad?.[ownerKey] : undefined)

	let showMax = $state(15)
	// A lazy owner paginates server-side ("Load more"), so when opened on its own it
	// shows all its already-loaded rows (no second, confusing client "Show more").
	// EXCEPT under "expand all" (collapseAll=false), which opens every visible owner at
	// once — rendering all of each would be thousands of rows and freeze the tab — so
	// there we cap to the client slice and let "Show more" reveal the rest per owner.
	let effectiveMax = $derived(
		isLazyOwner && ownerState != undefined && (isFolder(item) || isUser(item))
			? collapseAll
				? item.items.length
				: Math.min(item.items.length, showMax)
			: showMax
	)

	$effect(() => {
		// "Expand all" opens every node — EXCEPT top-level owners past the request cap:
		// opening those without loading would leave them empty and, worse, take two
		// clicks to load (first click just collapses). Keep them closed so one click
		// opens+loads them. The cap bounds the request burst at a large nbDisplayed.
		const cappedOwner =
			!collapseAll && depth === 0 && ownerKey != undefined && rootIndex >= EXPAND_ALL_LOAD_LIMIT
		const shouldOpen = !collapseAll && !cappedOwner
		opened = shouldOpen
		untrack(() => {
			if (ownerKey == undefined) return
			if (shouldOpen) {
				// Expand all opened+loads this in-cap owner.
				onExpandOwner?.(ownerKey)
			} else {
				// Closed here (collapse all, or a capped owner under expand all): untrack it
				// so a later reload doesn't re-fetch a hidden owner, recreating the fan-out
				// openOwners exists to prevent.
				onCollapseOwner?.(ownerKey)
			}
		})
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
		if (ownerKey != undefined) {
			if (opened) onExpandOwner?.(ownerKey)
			else onCollapseOwner?.(ownerKey)
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
						{#if isLazyOwner && !ownerState?.loaded}
							<!-- Lazy owner not expanded yet: its true item count is unknown until
							     loaded, so showing "(0 items)" would be misleading. -->
							&nbsp;
						{:else if isLazyOwner && ownerState?.hasMore}
							({item.items.length}+ items)
						{:else}
							({pluralize(item.items.length, ' item')})
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
						on:scriptChanged
						on:flowChanged
						on:appChanged
						on:rawAppChanged
						on:reload
						{showCode}
						depth={depth + 1}
					/>
				{/each}
				{#if effectiveMax < item.items.length}
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="text-center text-xs py-2 text-secondary cursor-pointer hover:text-primary"
						onclick={() => {
							showMax += Math.min(30, item.items.length - showMax)
						}}
					>
						Show more ({showMax}/{item.items.length})
					</div>
				{/if}
				{#if ownerKey != undefined}
					{#if ownerState?.loading && item.items.length === 0}
						<!-- Show the spinner only on the first load, when there's nothing yet. A
						     re-sort/re-filter re-fetch keeps the old rows visible and swaps them
						     in place, so flashing "Loading…" under them would just be noise. -->
						<div class="text-center text-xs py-2 text-secondary">Loading…</div>
					{:else if !ownerState?.loading && ownerState?.hasMore && effectiveMax >= item.items.length}
						<!-- Fetch the next server page only once every already-loaded row is shown
						     (in "expand all" the client "Show more" above reveals those first). -->
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="text-center text-xs py-2 text-primary cursor-pointer hover:text-emphasis"
							onclick={() => ownerKey != undefined && onExpandOwner?.(ownerKey, true)}
						>
							Load more in {ownerKey} ({ownerState?.count ?? item.items.length} loaded)
						</div>
					{/if}
				{/if}
			</div>
		{/if}
	</div>
{:else}
	<Item
		{item}
		{showCode}
		on:scriptChanged
		on:flowChanged
		on:appChanged
		on:rawAppChanged
		on:reload
		{depth}
	/>
{/if}
