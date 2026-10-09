<script lang="ts">
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import DropdownSubmenuItem from '$lib/components/DropdownSubmenuItem.svelte'
	import { Check, Loader2 } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarMenuElements, createDropdownMenu } from '@melt-ui/svelte'
	import type { Item } from '$lib/utils'
	import { Tooltip } from './meltComponents'

	interface Props {
		aiId?: string
		/** Already resolved and filtered; undefined while loading. */
		items: Item[] | undefined
		meltItem: MenubarMenuElements['item']
		builders?: ReturnType<typeof createDropdownMenu>['builders']
		close?: (afterClose?: () => void) => void
	}

	let { aiId, items, meltItem, builders, close }: Props = $props()
</script>

{#snippet menuItem(item: Item)}
	<MenuItem
		onClick={(e) => item?.action?.(e)}
		href={item?.href}
		target={item?.hrefTarget}
		disabled={item?.disabled}
		class={twMerge(
			'px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full',
			'data-[highlighted]:bg-surface-hover',
			'flex flex-row gap-2 items-center rounded-sm',
			// `pointer-events-none` lets a hover fall through the natively-disabled button to the
			// wrapper below, so a disabled item's `title` tooltip can still explain *why* it's disabled.
			item?.disabled && 'text-disabled cursor-not-allowed pointer-events-none',
			item?.type === 'delete' &&
				!item?.disabled &&
				'text-red-600 dark:text-red-400 data-[highlighted]:bg-red-500/10 dark:data-[highlighted]:bg-red-900/80 dark:data-[highlighted]:text-red-300 '
		)}
		item={meltItem}
		aiId={`${aiId ? `${aiId}-${item.displayName}` : undefined}`}
		aiDescription={item.displayName}
	>
		{#if item.icon}
			<item.icon size={14} color={item.iconColor} class="shrink-0" {...item.iconProps ?? {}} />
		{/if}
		<div class="grow min-w-0 text-left">
			<p
				title={item.disabled && item.tooltip ? undefined : item.displayName}
				class="truncate whitespace-nowrap"
			>
				{item.displayName}
			</p>
			{#if item.description}
				<p class="text-2xs text-secondary">{item.description}</p>
			{/if}
		</div>
		{@render item.extra?.()}
		{#if item.shortcut || item.selected || item.toggle !== undefined}
			<!-- Single trailing group so `shortcut` and `selected` can coexist:
			     two `ml-auto` siblings would collapse to one right-aligned item. -->
			<div class="ml-auto flex shrink-0 items-center gap-2">
				{#if item.shortcut}
					<span class="pl-4 text-2xs text-secondary">{item.shortcut}</span>
				{/if}
				{#if item.selected}
					<Check size={14} class="text-primary" />
				{/if}
				{#if item.toggle !== undefined}
					<!-- Indicator only: the click belongs to the row, so the switch must not
					     take it (nor answer for the row to a screen reader). -->
					<span class="pointer-events-none" aria-hidden="true">
						<Toggle size="2xs" checked={item.toggle} />
					</span>
				{/if}
			</div>
		{/if}
		{#if item.tooltip && !item.disabled}
			<!-- Enabled items get the rich ⓘ tooltip. Disabled items can't (a native disabled button
			     swallows subtree pointer events), so they fall back to the wrapper `title` below. -->
			<Tooltip>
				{#snippet text()}
					{item.tooltip}
				{/snippet}
			</Tooltip>
		{/if}
	</MenuItem>
{/snippet}

{#if items}
	<div class="flex flex-col">
		{#each items as item}
			{#if item.separatorTop}
				<div class="my-1 border-t border-border-light"></div>
			{/if}
			{#if (item.submenuItems || item.customSubmenu) && builders}
				<DropdownSubmenuItem {item} {builders} {meltItem} close={close ?? (() => {})} />
			{:else if item.disabled && item.tooltip}
				<!-- Wrapper carries the native `title`; the disabled button's `pointer-events-none`
				     lets the hover reach it so the user learns why the item is disabled. The ⓘ sits
				     outside the disabled button, the only place it can receive a hover. -->
				<div class="relative w-full">
					<div title={item.tooltip} class="w-full pr-6">
						{@render menuItem(item)}
					</div>
					<div class="absolute right-3 top-1/2 -translate-y-1/2 flex">
						<Tooltip>
							{#snippet text()}
								{item.tooltip}
							{/snippet}
						</Tooltip>
					</div>
				</div>
			{:else}
				{@render menuItem(item)}
			{/if}
		{/each}
	</div>
{:else}
	<Loader2 class="animate-spin mx-auto p-4" size={24} />
{/if}
