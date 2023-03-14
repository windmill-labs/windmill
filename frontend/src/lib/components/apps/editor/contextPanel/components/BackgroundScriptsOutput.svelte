<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import BackgroundScriptOutput from './BackgroundScriptOutput.svelte'

	const { app } = getContext<AppViewerContext>('AppViewerContext')

	export let expanded: boolean = false
</script>

{#if $app.hiddenInlineScripts.length > 0}
	<span class="text-xs font-bold p-2">Background scripts</span>
	<div class="border-t">
		{#each $app.hiddenInlineScripts as action, index}
			<BackgroundScriptOutput id={`bg_${index}`} name={action.name} {expanded} />
		{/each}
	</div>
{/if}
