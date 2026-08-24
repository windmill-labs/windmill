<script lang="ts">
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { PanelRight } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { Placement } from '@floating-ui/core'
	import type { FlowPanelDetachContext } from '../types'

	interface Props {
		/** 'control' borrows the graph control bar's cell styling; 'header' matches the icon
		 *  buttons beside it in the panel's card header. */
		variant: 'control' | 'header'
		placement?: Placement
	}

	let { variant, placement = 'bottom-end' }: Props = $props()

	const panelDetach = getContext<FlowPanelDetachContext | undefined>('flowPanelDetach')

	// Named options with a check, no per-row icons: whether the panel is attached is plain
	// from the layout, so the only thing worth spelling out is which of the three is active —
	// and Auto is the one name that doesn't say what it follows.
	const PANEL_PREFERENCES = [
		{
			value: 'auto' as const,
			displayName: 'Auto',
			tooltip: 'Follows the editor width: attached when there is room, detached when not'
		},
		{ value: 'docked' as const, displayName: 'Attached' },
		{ value: 'modal' as const, displayName: 'Detached' }
	]

	const items = $derived(
		PANEL_PREFERENCES.map((p) => ({
			displayName: p.displayName,
			tooltip: p.tooltip,
			selected: panelDetach?.preference() === p.value,
			action: () => panelDetach?.setPreference(p.value)
		}))
	)
</script>

{#if panelDetach?.enabled()}
	<DropdownV2
		{items}
		{placement}
		customWidth={220}
		class={variant === 'control' ? 'svelte-flow__controls-button !justify-center' : 'shrink-0'}
	>
		{#snippet buttonReplacement()}
			{#if variant === 'control'}
				<!-- The trigger itself carries the control-bar cell class. Wrapping a ControlButton
				     instead would nest a sized 32x30 box inside the trigger's own box, growing the
				     bar, and would take `:last-child` off the real last cell so its divider stayed. -->
				<span class="flex" title="Where the step panel opens">
					<PanelRight size="14" />
				</span>
			{:else}
				<Button
					nonCaptureEvent
					unifiedSize="sm"
					variant="subtle"
					iconOnly
					startIcon={{ icon: PanelRight }}
					title="Where the step panel opens"
				/>
			{/if}
		{/snippet}
	</DropdownV2>
{/if}
