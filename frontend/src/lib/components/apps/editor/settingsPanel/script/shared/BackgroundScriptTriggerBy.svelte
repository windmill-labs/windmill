<script lang="ts">
	import type { HiddenRunnable } from '$lib/components/apps/types'

	import { getDependencies } from '../utils'
	import ScriptTriggers from './ScriptTriggers.svelte'

	export let script: HiddenRunnable
	export let recomputeOnInputChanged: boolean | undefined = undefined

	$: isFrontend = script.type == 'runnableByName' && script.inlineScript?.language === 'frontend'
	$: triggerEvents = script.autoRefresh ? ['start', 'refresh'] : []
</script>

{#if script.type == 'runnableByName' && script.inlineScript}
	<ScriptTriggers
		bind:inlineScript={script.inlineScript}
		{triggerEvents}
		dependencies={getDependencies(script.fields)}
		{isFrontend}
		shoudlDisplayChangeEvents={recomputeOnInputChanged || isFrontend}
	/>
{:else if script.type === 'runnableByName'}
	<ScriptTriggers
		dependencies={getDependencies(script.fields)}
		{triggerEvents}
		{isFrontend}
		shoudlDisplayChangeEvents={recomputeOnInputChanged}
	/>
{/if}
