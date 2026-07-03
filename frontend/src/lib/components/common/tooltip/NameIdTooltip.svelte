<script lang="ts">
	import type { Placement } from '@floating-ui/core'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import CopyButton from '../button/CopyButton.svelte'

	// Rich hover tooltip for anything with a display name and a technical id
	// (workspaces, scripts, …): shows both plus a copy-id button. Wrap the
	// hoverable element as children; remove its native `title` so they don't
	// both pop.
	let {
		name,
		id,
		placement = 'bottom-start',
		disablePopup = false,
		class: className = '',
		children
	}: {
		name: string
		id: string
		placement?: Placement
		// Keeps the trigger rendered but suppresses the popup (e.g. when another
		// affordance like a collapsed-sidebar popover already covers hover).
		disablePopup?: boolean
		class?: string
		children?: import('svelte').Snippet
	} = $props()
</script>

<!-- openDelay keeps the tooltip from popping on casual pointer passes; closeDelay
     lets the pointer travel from the trigger into the tooltip to reach the copy
     button without the tooltip vanishing en route. -->
<Tooltip {placement} {disablePopup} openDelay={700} closeDelay={200} class={className}>
	{@render children?.()}
	{#snippet text()}
		<div class="flex flex-col gap-0.5 min-w-0">
			<div class="text-xs font-semibold text-emphasis truncate">{name}</div>
			<div class="flex items-center gap-1 min-w-0">
				<span class="font-mono text-2xs text-secondary truncate">{id}</span>
				<CopyButton value={id} />
			</div>
		</div>
	{/snippet}
</Tooltip>
