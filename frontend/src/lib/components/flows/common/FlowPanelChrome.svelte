<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { PanelRight, PictureInPicture2, X } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import type { FlowPanelDetachContext } from '../types'

	const panelDetach = getContext<FlowPanelDetachContext | undefined>('flowPanelDetach')

	// The detached modal draws no header of its own, so the header this sits in carries its
	// chrome too. onMount, not $effect: claim() increments (reads+writes) the claim count,
	// and a tracking effect would re-run on its own write.
	onMount(() => panelDetach?.claim())
</script>

{#if panelDetach?.visible()}
	<Button
		unifiedSize="sm"
		variant="subtle"
		iconOnly
		wrapperClasses="ml-2 shrink-0"
		startIcon={{ icon: PictureInPicture2 }}
		title="Detach into a modal"
		onClick={() => panelDetach.detach()}
	/>
{/if}
{#if panelDetach?.dockVisible() || panelDetach?.modalOpen()}
	<Button
		unifiedSize="sm"
		variant="subtle"
		iconOnly
		wrapperClasses="ml-2 shrink-0"
		startIcon={{ icon: PanelRight }}
		title="Dock to the right"
		onClick={() => panelDetach.dock()}
	/>
{/if}
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
