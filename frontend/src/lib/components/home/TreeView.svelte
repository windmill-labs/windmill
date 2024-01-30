<svelte:options immutable={true} />

<script lang="ts">
	import { ChevronDown, ChevronUp, Folder, FolderTree, User } from 'lucide-svelte'
	import Item from './Item.svelte'
	import type { FolderItem, ItemType, UserItem } from './treeViewUtils'
	import { twMerge } from 'tailwind-merge'
	import { pluralize } from '$lib/utils'

	export let item: ItemType | FolderItem | UserItem

	export let collapseAll: boolean
	export let depth: number = 0
	export let showCode: (path: string, summary: string) => void
	export let isSearching: boolean = false

	const isFolder = (i: any): i is FolderItem => i && 'folderName' in i
	const isUser = (i: any): i is UserItem => i && 'username' in i

	let opened: boolean = true

	$: toggleOpened(collapseAll)

	function toggleOpened(collapseAll: boolean) {
		opened = !collapseAll
	}

	let showMax = 15
</script>

{#if isFolder(item)}
	<div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			on:click={() => (opened = !opened)}
			class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
		>
			<div
				class={twMerge('flex flex-row items-center gap-4 text-sm font-semibold')}
				style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
			>
				<div
					class=" rounded-md p-1 flex justify-center items-center border bg-gray-50 border-gray-200 dark:bg-transparent dark:border-gray-900"
				>
					{#if depth === 0}
						<Folder size={20} color="#aaa" />
					{:else}
						<FolderTree size={20} color="#aaa" />
					{/if}
				</div>

				<div>
					{#if depth === 0}f/{/if}{item.folderName}
					<div class="text-2xs font-normal text-secondary whitespace-nowrap">
						({pluralize(item.items.length, ' item')})
					</div>
				</div>
			</div>
			<button class="w-full flex flex-row-reverse">
				{#if opened}
					<ChevronUp size={20} />
				{:else}
					<ChevronDown size={20} />
				{/if}
			</button>
		</div>
		{#if opened || isSearching}
			<div>
				{#each item.items.slice(0, showMax) as subItem ((subItem['path'] ? subItem['type'] + '__' + subItem['path'] : undefined) ?? 'folder__' + subItem['folderName'])}
					<svelte:self
						{isSearching}
						{collapseAll}
						item={subItem}
						on:scriptChanged
						on:flowChanged
						on:appChanged
						on:rawAppChanged
						on:reload
						depth={depth + 1}
					/>
				{/each}
				{#if showMax < item.items.length}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						class="text-center text-sm text-secondary cursor-pointer hover:text-primary"
						on:click={() => {
							if (isFolder(item)) {
								showMax += Math.min(30, item.items.length - showMax)
								showMax = showMax
							}
						}}
					>
						Show more ({showMax}/{item.items.length})
					</div>
				{/if}
			</div>
		{/if}
	</div>
{:else if isUser(item)}
	<div>
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			on:click={() => (opened = !opened)}
			class="px-4 py-2 border-b w-full flex flex-row items-center justify-between cursor-pointer"
		>
			<div
				class={twMerge('flex flex-row items-center gap-4 text-sm font-semibold')}
				style={depth > 0 ? `padding-left: ${depth * 16}px;` : ''}
			>
				<div
					class=" rounded-md p-1 flex justify-center items-center border bg-gray-50 border-gray-200 dark:bg-transparent dark:border-gray-900"
				>
					<User size={20} color="#aaa" />
				</div>

				<div>
					u/{item.username}
					<div class="text-2xs font-normal text-secondary whitespace-nowrap"
						>({pluralize(item.items.length, ' item')})</div
					>
				</div>
			</div>
			<div class="w-full flex flex-row-reverse">
				{#if opened}
					<ChevronUp size={20} />
				{:else}
					<ChevronDown size={20} />
				{/if}
			</div>
		</div>
		{#if opened || isSearching}
			<div>
				{#each item.items.slice(0, showMax) as subItem ((subItem['path'] ? subItem['type'] + '__' + subItem['path'] : undefined) ?? 'folder__' + subItem['folderName'])}
					<svelte:self
						{collapseAll}
						item={subItem}
						on:scriptChanged
						on:flowChanged
						on:appChanged
						on:rawAppChanged
						on:reload
						depth={depth + 1}
					/>
				{/each}
				{#if showMax < item.items.length}
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						class="text-center text-sm text-secondary cursor-pointer py-2 hover:text-primary"
						on:click={() => {
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
