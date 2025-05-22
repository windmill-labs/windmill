<script lang="ts">
	import { MenuItem } from '$lib/components/meltComponents'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import type { Item } from '$lib/utils'
	export let items: Item[] | (() => Item[]) | (() => Promise<Item[]>) = []
	export let meltItem: MenubarMenuElements['item']

	let computedItems: Item[] | undefined = undefined
	async function computeItems() {
		if (typeof items === 'function') {
			computedItems = ((await items()) ?? []).filter((item) => !item.hide)
		} else {
			computedItems = items.filter((item) => !item.hide)
		}
	}

	computeItems()
</script>

{#if computedItems}
	<div class="flex flex-col">
		{#each computedItems ?? [] as item}
			<MenuItem
				on:click={(e) => item?.action?.(e)}
				href={item?.href}
				disabled={item?.disabled}
				class={twMerge(
					'px-4 py-2 text-primary font-semibold hover:bg-surface-hover cursor-pointer text-xs transition-all',
					'data-[highlighted]:bg-surface-hover',
					'flex flex-row gap-2 items-center',
					item?.disabled && 'text-gray-400 cursor-not-allowed',
					item?.type === 'delete' &&
						!item?.disabled &&
						'text-red-500 hover:bg-red-100 hover:text-red-500 data-[highlighted]:text-red-500 data-[highlighted]:bg-red-100'
				)}
				item={meltItem}
			>
				{#if item.icon}
					<svelte:component this={item.icon} size={14} color={item.iconColor} />
				{/if}
				{item.displayName}
			</MenuItem>
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin mx-auto p-4" size={24} />
{/if}
