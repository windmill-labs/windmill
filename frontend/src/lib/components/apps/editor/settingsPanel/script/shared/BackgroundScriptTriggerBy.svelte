<script lang="ts">
	import type { HiddenRunnable } from '$lib/components/apps/types'

	import { getDependencies } from '../utils'
	import ScriptTriggers from './ScriptTriggers.svelte'

	interface Props {
		script: HiddenRunnable
		recomputeOnInputChanged?: boolean | undefined
		id: string
	}

	let { script = $bindable(), recomputeOnInputChanged = undefined, id }: Props = $props()

	let isFrontend = $derived(
		script.type == 'runnableByName' && script.inlineScript?.language === 'frontend'
	)
	let triggerEvents = $derived(script.autoRefresh ? ['start', 'refresh'] : [])
</script>

{#if script.type == 'runnableByName' && script.inlineScript}
	<ScriptTriggers
		{id}
		bind:inlineScript={script.inlineScript}
		{triggerEvents}
		dependencies={getDependencies(script.fields)}
		{isFrontend}
		shoudlDisplayChangeEvents={recomputeOnInputChanged || isFrontend}
	/>
{:else if script.type === 'runnableByName'}
	<ScriptTriggers
		{id}
		dependencies={getDependencies(script.fields)}
		{triggerEvents}
		{isFrontend}
		shoudlDisplayChangeEvents={recomputeOnInputChanged}
	/>
{/if}
