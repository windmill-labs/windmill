<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import BackgroundScriptOutput from './BackgroundScriptOutput.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
</script>

{#each $app.hiddenInlineScripts as action, index}
	{#if !action.hidden}
		<BackgroundScriptOutput id={`bg_${index}`} name={action.name} first={index === 0} />
	{/if}
{/each}
