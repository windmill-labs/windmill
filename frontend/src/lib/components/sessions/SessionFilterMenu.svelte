<script lang="ts">
	import { untrack } from 'svelte'
	import { melt } from '@melt-ui/svelte'
	import type { MenubarMenuBuilders } from '@melt-ui/svelte'
	import { ChevronRight, Filter } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import Toggle from '$lib/components/Toggle.svelte'

	interface Props {
		// Submenu builders from the enclosing melt Menu — createSubmenu must be
		// called against this specific menu instance (same pattern as DropdownSubmenuItem).
		builders: MenubarMenuBuilders
		showArchived: boolean
		showAllWorkspaces: boolean
		archivedCount: number
	}

	let {
		builders,
		showArchived = $bindable(),
		showAllWorkspaces = $bindable(),
		archivedCount
	}: Props = $props()

	const {
		elements: { subTrigger, subMenu },
		states: { subOpen }
	} = untrack(() => builders).createSubmenu()

	// Count of active (non-default) filters, surfaced as a subtle badge on the
	// trigger so an applied filter is visible without opening the submenu.
	let activeCount = $derived((showArchived ? 1 : 0) + (showAllWorkspaces ? 1 : 0))
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
		<!-- Rendered as plain content (not a MenuItem) so toggling a filter keeps
		     the submenu open instead of selecting-and-closing the whole menu. -->
		<div class="px-3 py-1.5 flex flex-col gap-2">
			<div class="flex flex-col gap-0.5">
				<Toggle
					bind:checked={showAllWorkspaces}
					size="xs"
					options={{ right: 'Show all workspaces' }}
				/>
				<span class="text-2xs text-tertiary pl-1">Include sessions from every workspace.</span>
			</div>
			<div class="flex flex-col gap-0.5">
				<Toggle bind:checked={showArchived} size="xs" options={{ right: 'Show archived' }} />
				{#if archivedCount > 0}
					<span class="text-2xs text-tertiary pl-1">
						{archivedCount} archived session{archivedCount === 1 ? '' : 's'}
					</span>
				{/if}
			</div>
		</div>
	</div>
{/if}
