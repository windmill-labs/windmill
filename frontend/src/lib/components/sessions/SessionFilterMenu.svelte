<script lang="ts">
	import { untrack } from 'svelte'
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuBuilders } from '@melt-ui/svelte'
	import { Check, ChevronRight, Filter } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Toggle from '$lib/components/Toggle.svelte'
	import { GROUP_BY_OPTIONS, LAST_ACTIVITY_OPTIONS, type GroupBy } from './sessionFilters'

	interface Props {
		// Submenu builders from the enclosing melt Menu — createSubmenu must be
		// called against this specific menu instance (same pattern as DropdownSubmenuItem).
		builders: MenubarMenuBuilders
		showArchived: boolean
		archivedCount: number
		lastActivityDays: number
		groupBy: GroupBy
	}

	let {
		builders,
		showArchived = $bindable(),
		archivedCount,
		lastActivityDays = $bindable(),
		groupBy = $bindable()
	}: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = untrack(() => builders).createSubmenu()

	// Count of active (non-default) filters, surfaced as a subtle badge on the
	// trigger so an applied filter is visible without opening the submenu.
	// Grouping is left out: it reorders the list rather than hiding anything.
	let activeCount = $derived((showArchived ? 1 : 0) + (lastActivityDays > 0 ? 1 : 0))

	const optionRow = twMerge(
		'px-3 py-1.5 w-full text-left text-xs font-normal text-secondary',
		'flex flex-row items-center gap-2 rounded-sm hover:bg-surface-hover hover:text-primary'
	)
</script>

<button
	use:melt={$subTrigger}
	class={twMerge(
		'px-3 py-1.5 w-full text-left text-xs font-normal text-secondary',
		'flex flex-row items-center gap-2 rounded-sm',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)}
>
	<Filter size={14} class="shrink-0 text-tertiary" />
	<span class="grow">Filter</span>
	{#if activeCount > 0}
		<span class="text-2xs text-tertiary tabular-nums">{activeCount}</span>
	{/if}
	<ChevronRight size={14} class="shrink-0 text-tertiary" />
</button>

{#if $subOpen}
	<div
		use:melt={$subMenu}
		class="z-[6000] w-48 bg-surface dark:border rounded-md shadow-md focus:outline-none py-1"
	>
		<!-- Rendered as plain content (not MenuItems) so picking a filter keeps the
		     submenu open instead of selecting-and-closing the whole menu. -->
		<div class="px-3 py-1.5 flex flex-col gap-0.5">
			<Toggle bind:checked={showArchived} size="xs" options={{ right: 'Show archived' }} />
			{#if archivedCount > 0}
				<span class="text-2xs text-tertiary pl-1">
					{archivedCount} archived session{archivedCount === 1 ? '' : 's'}
				</span>
			{/if}
		</div>
		<div class="my-1 border-t border-border-light"></div>
		<div class="px-3 pb-0.5 text-3xs text-tertiary">Last activity</div>
		{#each LAST_ACTIVITY_OPTIONS as option (option.days)}
			<button type="button" class={optionRow} onclick={() => (lastActivityDays = option.days)}>
				<span class="grow truncate">{option.label}</span>
				{#if lastActivityDays === option.days}
					<Check size={14} class="shrink-0 text-primary" />
				{/if}
			</button>
		{/each}
		<div class="my-1 border-t border-border-light"></div>
		<div class="px-3 pb-0.5 text-3xs text-tertiary">Group by</div>
		{#each GROUP_BY_OPTIONS as option (option.value)}
			<button type="button" class={optionRow} onclick={() => (groupBy = option.value)}>
				<span class="grow truncate">{option.label}</span>
				{#if groupBy === option.value}
					<Check size={14} class="shrink-0 text-primary" />
				{/if}
			</button>
		{/each}
	</div>
{/if}
