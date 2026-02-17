<script lang="ts">
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import { melt } from '@melt-ui/svelte'
	import { twMerge } from 'tailwind-merge'
	import { ChevronRight } from 'lucide-svelte'
	import type { Item } from '$lib/utils'
	import type { MenubarMenuElements } from '@melt-ui/svelte'
	import { Tooltip } from './meltComponents'

	interface Props {
		item: Item
		builders: any
		meltItem: MenubarMenuElements['item']
	}

	let { item, builders, meltItem }: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = builders.createSubmenu()

	let subItems = $derived((item.submenuItems ?? []).filter((i) => !i.hide))
</script>

<button
	use:melt={$subTrigger}
	class={twMerge(
		'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
		'data-[highlighted]:bg-surface-hover',
		'flex flex-row gap-2 items-center rounded-sm'
	)}
>
	{#if item.icon}
		<item.icon size={14} color={item.iconColor} class="shrink-0" />
	{/if}
	<p class="truncate grow min-w-0 whitespace-nowrap text-left">
		{item.displayName}
	</p>
	{@render item.extra?.()}
	<ChevronRight size={14} class="ml-auto shrink-0 text-tertiary" />
</button>

{#if $subOpen}
	<div
		use:melt={$subMenu}
		class="z-[6000] bg-surface-tertiary dark:border w-48 origin-top-right rounded-lg shadow-lg focus:outline-none overflow-y-auto py-1"
	>
		{#each subItems as subItem}
			{#if subItem.separatorTop}
				<div class="my-1 border-t border-border-light"></div>
			{/if}
			<MenuItem
				onClick={(e) => subItem?.action?.(e)}
				href={subItem?.href}
				target={subItem?.hrefTarget}
				disabled={subItem?.disabled}
				class={twMerge(
					'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
					'data-[highlighted]:bg-surface-hover',
					'flex flex-row gap-2 items-center rounded-sm',
					subItem?.disabled && 'text-disabled cursor-not-allowed'
				)}
				item={meltItem}
			>
				{#if subItem.icon}
					<subItem.icon size={14} color={subItem.iconColor} class="shrink-0" />
				{/if}
				<p title={subItem.displayName} class="truncate grow min-w-0 whitespace-nowrap text-left">
					{subItem.displayName}
				</p>
				{@render subItem.extra?.()}
				{#if subItem.tooltip}
					<Tooltip>
						{#snippet text()}
							{subItem.tooltip}
						{/snippet}
					</Tooltip>
				{/if}
			</MenuItem>
		{/each}
	</div>
{/if}
