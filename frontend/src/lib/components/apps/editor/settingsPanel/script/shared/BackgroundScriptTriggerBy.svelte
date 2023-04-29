<script lang="ts">
	import type { HiddenInlineScript } from '$lib/components/apps/types'

	import { getDependencies } from '../utils'
	import ScriptTriggers from './ScriptTriggers.svelte'

	export let script: HiddenInlineScript
	export let recomputeOnInputChanged: boolean | undefined = undefined

	$: isFrontend = script.inlineScript?.language === 'frontend'
	$: triggerEvents = script.autoRefresh ? ['start', 'refresh'] : []
</script>

{#if script.inlineScript}
	<ScriptTriggers
		bind:inlineScript={script.inlineScript}
		{triggerEvents}
		dependencies={getDependencies(script.fields)}
		{isFrontend}
		shoudlDisplayChangeEvents={recomputeOnInputChanged || isFrontend}
	/>
{/if}
