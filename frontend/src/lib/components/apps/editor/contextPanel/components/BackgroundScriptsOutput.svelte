<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { getContext } from 'svelte'
	import BackgroundScriptOutput from './BackgroundScriptOutput.svelte'
	import { BG_PREFIX } from '$lib/components/apps/utils'

	const { app } = getContext<AppViewerContext>('AppViewerContext')
</script>

{#each app.hiddenInlineScripts ?? [] as action, index}
	{#if !action.hidden}
		<BackgroundScriptOutput id={BG_PREFIX + index} name={action.name} first={index === 0} />
	{/if}
{/each}
