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
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean }
		>
		onExpandOwner?: (owner: string, more?: boolean) => void
		onCollapseOwner?: (owner: string) => void
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
		onCollapseOwner
	}: Props = $props()

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
	// A lazy owner paginates server-side ("Load more"), so show all its already-loaded
	// items rather than the tight client slice (which would add a second, confusing
	// "Show more" button).
	let effectiveMax = $derived(
		isLazyOwner && ownerState != undefined && (isFolder(item) || isUser(item))
			? item.items.length
			: showMax
	)

	$effect(() => {
		const willOpen = !collapseAll
		opened = willOpen
		// "Collapse all" closes this node without a manual click, so untrack the owner
		// here too — otherwise it lingers in openOwners and a later reload re-fetches
		// every hidden owner, recreating the fan-out openOwners exists to prevent.
		untrack(() => {
			if (!willOpen && ownerKey != undefined) onCollapseOwner?.(ownerKey)
		})
	})
	// Deliberately NOT auto-loading an owner's items when it opens via collapseAll:
	// with every folder and user injected as a top-level node, "expand all" would
	// otherwise fire one request per owner (thousands on a large workspace). An owner
	// loads only on an explicit click (see toggleOwner).
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
					{:else if !ownerState?.loading && ownerState?.hasMore}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="text-center text-xs py-2 text-primary cursor-pointer hover:text-emphasis"
							onclick={() => ownerKey != undefined && onExpandOwner?.(ownerKey, true)}
						>
							Load more
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
