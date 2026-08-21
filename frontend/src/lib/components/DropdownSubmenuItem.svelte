<script lang="ts">
	import { untrack } from 'svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import { melt } from '@melt-ui/svelte'
	import { twMerge } from 'tailwind-merge'
	import { Check, ChevronRight } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import type { Item } from '$lib/utils'
	import type { MenubarMenuElements, createDropdownMenu } from '@melt-ui/svelte'
	import { Tooltip } from './meltComponents'

	interface Props {
		item: Item
		builders: ReturnType<typeof createDropdownMenu>['builders']
		meltItem: MenubarMenuElements['item']
	}

	let { item, builders, meltItem }: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = untrack(() => builders).createSubmenu()

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
			{@render subMenuItem(subItem)}
		{/each}
	</div>
{/if}

{#snippet subMenuItem(subItem: Item)}
	{#if subItem.disabled && subItem.tooltip}
		<!-- Wrapper carries the native `title`: the disabled button swallows the pointer events
		     the ⓘ tooltip would need, and its `pointer-events-none` lets the hover reach here. -->
		<div title={subItem.tooltip} class="w-full">
			{@render row(subItem)}
		</div>
	{:else}
		{@render row(subItem)}
	{/if}
{/snippet}

{#snippet row(subItem: Item)}
	<MenuItem
		onClick={(e) => subItem?.action?.(e)}
		href={subItem?.href}
		target={subItem?.hrefTarget}
		disabled={subItem?.disabled}
		class={twMerge(
			'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
			'data-[highlighted]:bg-surface-hover',
			'flex flex-row gap-2 items-center rounded-sm',
			subItem?.disabled && 'text-disabled cursor-not-allowed',
			subItem?.disabled && subItem?.tooltip && 'pointer-events-none'
		)}
		item={meltItem}
	>
		{#if subItem.icon}
			<subItem.icon size={14} color={subItem.iconColor} class="shrink-0" {...subItem.iconProps ?? {}} />
		{/if}
		<p
			title={subItem.disabled && subItem.tooltip ? undefined : subItem.displayName}
			class="truncate grow min-w-0 whitespace-nowrap text-left"
		>
			{subItem.displayName}
		</p>
		{@render subItem.extra?.()}
		{#if subItem.shortcut || subItem.selected || subItem.toggle !== undefined}
			<div class="ml-auto flex shrink-0 items-center gap-2">
				{#if subItem.shortcut}
					<span class="pl-4 text-2xs text-secondary">{subItem.shortcut}</span>
				{/if}
				{#if subItem.selected}
					<Check size={14} class="text-primary" />
				{/if}
				{#if subItem.toggle !== undefined}
					<!-- Indicator only: the click belongs to the row, so the switch must not
					     take it (nor answer for the row to a screen reader). -->
					<span class="pointer-events-none" aria-hidden="true">
						<Toggle size="2xs" checked={subItem.toggle} />
					</span>
				{/if}
			</div>
		{/if}
		{#if subItem.tooltip && !subItem.disabled}
			<Tooltip>
				{#snippet text()}
					{subItem.tooltip}
				{/snippet}
			</Tooltip>
		{/if}
	</MenuItem>
{/snippet}
