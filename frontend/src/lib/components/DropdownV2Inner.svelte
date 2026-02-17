<script lang="ts">
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import DropdownSubmenuItem from '$lib/components/DropdownSubmenuItem.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import type { Item } from '$lib/utils'
	import { Tooltip } from './meltComponents'

	interface Props {
		aiId?: string
		items?: Item[] | (() => Item[]) | (() => Promise<Item[]>)
		meltItem: MenubarMenuElements['item']
		builders?: any
	}

	let { aiId, items = [], meltItem, builders }: Props = $props()

	let computedItems: Item[] | undefined = $state(undefined)
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
			{#if item.separatorTop}
				<div class="my-1 border-t border-border-light"></div>
			{/if}
			{#if item.submenuItems && builders}
				<DropdownSubmenuItem {item} {builders} {meltItem} />
			{:else}
				<MenuItem
					onClick={(e) => item?.action?.(e)}
					href={item?.href}
					target={item?.hrefTarget}
					disabled={item?.disabled}
					class={twMerge(
						'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
						'data-[highlighted]:bg-surface-hover',
						'flex flex-row gap-2 items-center rounded-sm',
						item?.disabled && 'text-disabled cursor-not-allowed',
						item?.type === 'delete' &&
							!item?.disabled &&
							'text-red-600 dark:text-red-400 data-[highlighted]:bg-red-500/10 dark:data-[highlighted]:bg-red-900/80 dark:data-[highlighted]:text-red-300 '
					)}
					item={meltItem}
					aiId={`${aiId ? `${aiId}-${item.displayName}` : undefined}`}
					aiDescription={item.displayName}
				>
					{#if item.icon}
						<item.icon size={14} color={item.iconColor} class="shrink-0" />
					{/if}
					<p title={item.displayName} class="truncate grow min-w-0 whitespace-nowrap text-left">
						{item.displayName}
					</p>
					{@render item.extra?.()}
					{#if item.tooltip}
						<Tooltip>
							{#snippet text()}
								{item.tooltip}
							{/snippet}
						</Tooltip>
					{/if}
				</MenuItem>
			{/if}
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin mx-auto p-4" size={24} />
{/if}
