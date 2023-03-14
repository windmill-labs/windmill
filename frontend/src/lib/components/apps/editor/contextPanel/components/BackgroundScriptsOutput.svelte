<script lang="ts">
	import type { AppViewerContext, GridItem } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import OutputHeader from './OutputHeader.svelte'
	import BackgroundScriptOutput from './BackgroundScriptOutput.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let expanded: boolean = false

	let open: boolean = false
	let manuallyOpen = false

	$: if (expanded) {
		manuallyOpen = true
	} else {
		manuallyOpen = false
	}
</script>

<OutputHeader
	open={open || manuallyOpen}
	{manuallyOpen}
	on:click={() => {
		manuallyOpen = !manuallyOpen
	}}
	id="BG"
	name={'Background scripts'}
	color="blue"
/>

{#if open || manuallyOpen}
	<div class="py-1 ml-2 border">
		{#each $app.hiddenInlineScripts as action}
			<BackgroundScriptOutput id={action.name} {expanded} />
		{/each}
	</div>
{/if}
