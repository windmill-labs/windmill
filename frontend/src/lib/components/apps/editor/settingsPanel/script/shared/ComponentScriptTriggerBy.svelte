<script lang="ts">
	import { isRunnableByName, type ResultAppInput } from '$lib/components/apps/inputType'
	import type { AppComponent } from '../../../component'
	import { getAllTriggerEvents, isTriggerable, getDependencies } from '../utils'

	import ScriptTriggers from './ScriptTriggers.svelte'

	interface Props {
		appComponent: AppComponent
		appInput: ResultAppInput
	}

	let { appComponent, appInput = $bindable() }: Props = $props()

	let triggerEvents = $derived(getAllTriggerEvents(appComponent, appInput.autoRefresh))
	let isFrontend = $derived(
		isRunnableByName(appInput.runnable) && appInput.runnable?.inlineScript?.language === 'frontend'
	)
	let shoudlDisplayChangeEvents = $derived(
		appInput.recomputeOnInputChanged && !isTriggerable(appComponent.type)
	)
</script>

{#if isRunnableByName(appInput.runnable)}
	<ScriptTriggers
		id={appComponent.id}
		bind:inlineScript={appInput.runnable.inlineScript}
		dependencies={getDependencies(appInput.fields)}
		{isFrontend}
		{triggerEvents}
		{shoudlDisplayChangeEvents}
	/>
{:else}
	<ScriptTriggers
		id={appComponent.id}
		dependencies={getDependencies(appInput.fields)}
		{triggerEvents}
		{isFrontend}
		{shoudlDisplayChangeEvents}
	/>
{/if}
