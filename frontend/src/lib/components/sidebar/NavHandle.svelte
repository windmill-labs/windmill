<script lang="ts">
	import { Building } from 'lucide-svelte'
	import { workspaceColor } from '$lib/stores'
	import { getContrastTextColor } from '$lib/utils'
	import { Tooltip } from '$lib/components/meltComponents'
	import MenuButton from './MenuButton.svelte'
	import { navHandleSlot } from './navHandlePlacement.svelte'

	// The detached sidebar's trigger, floating over the page's top-left corner. Hover or click
	// opens the card; docking is the card header's own button.
	const iconColor = $derived(getContrastTextColor($workspaceColor))
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div data-nav-handle onmouseenter={() => navHandleSlot.open()}>
	<Tooltip class="flex" placement="right" small>
		<MenuButton
			class="!text-xs"
			buttonClass="!pl-3.5 !pr-1 !w-auto"
			icon={Building}
			isCollapsed={false}
			lightMode
			color={$workspaceColor ?? 'rgb(var(--color-surface-sunken))'}
			iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
			ariaLabel="Open sidebar"
			on:click={() => navHandleSlot.open()}
		/>
		{#snippet text()}Open sidebar{/snippet}
	</Tooltip>
</div>
