<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import type { AppComponent } from '../../../component'
	import { getAllTriggerEvents, isTriggerable, getDependencies } from '../utils'

	import ScriptTriggers from './ScriptTriggers.svelte'

	export let appComponent: AppComponent
	export let appInput: ResultAppInput

	$: triggerEvents = getAllTriggerEvents(appComponent, appInput.autoRefresh)
	$: isFrontend =
		appInput.runnable?.type == 'runnableByName' &&
		appInput.runnable?.inlineScript?.language === 'frontend'
	$: shoudlDisplayChangeEvents =
		appInput.recomputeOnInputChanged && !isTriggerable(appComponent.type)
</script>

{#if appInput?.runnable?.type === 'runnableByName'}
	<ScriptTriggers
		bind:inlineScript={appInput.runnable.inlineScript}
		dependencies={getDependencies(appInput.fields)}
		{isFrontend}
		{triggerEvents}
		{shoudlDisplayChangeEvents}
	/>
{:else}
	<ScriptTriggers
		dependencies={getDependencies(appInput.fields)}
		{triggerEvents}
		{isFrontend}
		{shoudlDisplayChangeEvents}
	/>
{/if}
