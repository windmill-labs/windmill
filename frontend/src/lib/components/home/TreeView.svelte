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
		// Lazy per-folder loading (top-level folders only): expanding a folder loads
		// its items on demand and paginates within it.
		folderLoad?: Record<
			string,
			{ cursor?: string; hasMore: boolean; loading: boolean; loaded: boolean }
		>
		onExpandFolder?: (folder: string, more?: boolean) => void
	}

	let {
		item,
		collapseAll,
		depth = 0,
		showCode,
		isSearching = false,
		pipelineFolders,
		folderLoad,
		onExpandFolder
	}: Props = $props()

	const isFolderItem = (i: typeof item): i is FolderItem => i && 'folderName' in i
	// Hidden while searching: pipelines aren't part of the text filter (the list
	// view hides their rows on a query too), so a folder matching the search
	// shouldn't surface an unrelated Pipeline row.
	let hasPipeline = $derived(
		depth === 0 &&
			!isSearching &&
			isFolderItem(item) &&
			(pipelineFolders?.has(item.folderName) ?? false)
	)

	const isFolder = isFolderItem
	const isUser = (i: typeof item): i is UserItem => i && 'username' in i

	let opened: boolean = $state(true)

	let showMax = $state(15)
	// A lazy top-level folder paginates server-side ("Load more in this folder"),
	// so show all of its already-loaded items rather than the tight client slice
	// (which would add a second, confusing "Show more" button).
	let effectiveMax = $derived(
		depth === 0 && isFolderItem(item) && folderLoad?.[item.folderName] != undefined
			? item.items.length
			: showMax
	)
	$effect(() => {
		opened = !collapseAll
		// Expand-all opens folders; lazy-load a top-level folder's items when it
		// opens. untrack so this only re-runs on collapseAll (not on item reloads,
		// which would otherwise re-open a manually collapsed folder).
		untrack(() => {
			if (opened && depth === 0 && isFolder(item)) onExpandFolder?.(item.folderName)
		})
	})
</script>

{#if isFolder(item)}
	<div>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onclick={() => {
				opened = !opened
				// Top-level folder: load its items on demand the first time it opens.
				if (opened && depth === 0 && isFolder(item)) onExpandFolder?.(item.folderName)
			}}
			class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
		>
			<div
				class={twMerge('flex flex-row items-center gap-4 text-sm font-semibold')}
				style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
			>
				<div class="flex justify-center items-center">
					{#if depth === 0}
						<Folder size={16} class="text-secondary" />
					{:else}
						<FolderTree size={16} class="text-secondary" />
					{/if}
				</div>

				<div>
					<span class="whitespace-nowrap text-xs text-emphasis font-semibold"
						>{#if depth === 0}f/{/if}{item.folderName}</span
					>
					<div class="text-2xs font-normal text-secondary whitespace-nowrap">
						({pluralize(item.items.length, ' item')})
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
							if (isFolder(item)) {
								showMax += Math.min(30, item.items.length - showMax)
								showMax = showMax
							}
						}}
					>
						Show more ({showMax}/{item.items.length})
					</div>
				{/if}
				{#if depth === 0 && isFolder(item)}
					{#if folderLoad?.[item.folderName]?.loading}
						<div class="text-center text-xs py-2 text-secondary">Loading…</div>
					{:else if folderLoad?.[item.folderName]?.hasMore}
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							class="text-center text-xs py-2 text-primary cursor-pointer hover:text-emphasis"
							onclick={() => isFolder(item) && onExpandFolder?.(item.folderName, true)}
						>
							Load more in this folder
						</div>
					{/if}
				{/if}
			</div>
		{/if}
	</div>
{:else if isUser(item)}
	<div>
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			onclick={() => (opened = !opened)}
			class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
		>
			<div
				class={twMerge('flex flex-row items-center gap-4 text-sm font-semibold')}
				style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
			>
				<div class="flex justify-center items-center">
					<User size={16} class="text-secondary" />
				</div>

				<div>
					<span class="whitespace-nowrap text-xs text-emphasis font-semibold"
						>u/{item.username}</span
					>
					<div class="text-2xs font-normal text-secondary whitespace-nowrap"
						>({pluralize(item.items.length, ' item')})</div
					>
				</div>
			</div>
			<div class="w-full flex flex-row-reverse">
				{#if opened}
					<ChevronUp size={16} />
				{:else}
					<ChevronDown size={16} />
				{/if}
			</div>
		</div>
		{#if opened || isSearching}
			<div>
				{#each item.items.slice(0, effectiveMax) as subItem, index ((subItem['path'] ? subItem['type'] + '__' + subItem['path'] + '__' + index : undefined) ?? 'folder__' + subItem['folderName'] + '__' + index)}
					<TreeView
						{collapseAll}
						item={subItem}
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
						class="text-center text-xs text-secondary cursor-pointer py-2 hover:text-primary"
						onclick={() => {
							if (isUser(item)) {
								showMax += Math.min(30, item.items.length - showMax)
							}
						}}
					>
						Show more ({showMax}/{item.items.length})
					</div>
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
