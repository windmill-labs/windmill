<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { X } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import type { FlowPanelDetachContext } from '../types'
	import FlowPanelPlacementPicker from './FlowPanelPlacementPicker.svelte'

	const panelDetach = getContext<FlowPanelDetachContext | undefined>('flowPanelDetach')

	// The detached modal draws no header of its own, so the header this sits in carries its
	// chrome too. onMount, not $effect: claim() increments (reads+writes) the claim count,
	// and a tracking effect would re-run on its own write.
	onMount(() => panelDetach?.claim())
</script>

<div class="ml-2 flex shrink-0 items-center">
	<FlowPanelPlacementPicker variant="header" />
</div>
{#if panelDetach?.modalOpen()}
	<Button
		unifiedSize="sm"
		variant="subtle"
		iconOnly
		wrapperClasses="shrink-0"
		startIcon={{ icon: X }}
		title="Close"
		onClick={() => panelDetach.close()}
	/>
{/if}
