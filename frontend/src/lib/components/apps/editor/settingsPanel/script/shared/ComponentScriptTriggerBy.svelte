<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { classNames } from '$lib/utils'
	import type { AppComponent } from '../../../component'
	import { getAllTriggerEvents, isTriggerable, getDependencies } from '../utils'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'

	export let appComponent: AppComponent
	export let appInput: ResultAppInput

	$: triggerEvents = getAllTriggerEvents(appComponent, appInput.autoRefresh)
	$: changeEvents = getDependencies(appInput.fields)
	$: hasNoTriggers =
		triggerEvents.length === 0 && (changeEvents.length === 0 || !appInput.recomputeOnInputChanged)

	const badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
	const colors = {
		green: 'text-green-800 border-green-600 bg-green-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100'
	}
</script>

<ScriptSettingsSection title="Triggers">
	{#if hasNoTriggers}
		<Alert type="warning" title="No triggers" size="xs">
			This script has no triggers. It will never run.
		</Alert>
	{:else}
		{#if triggerEvents.length > 0}
			<div class="text-xs font-semibold text-slate-800 mb-1">Events</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each triggerEvents as triggerEvent}
					<span class={classNames(badgeClass, colors['green'])}>{triggerEvent}</span>
				{/each}
			</div>
		{/if}
		{#if changeEvents.length > 0 && appInput.recomputeOnInputChanged && !isTriggerable(appComponent.type)}
			<div class="text-xs font-semibold text-slate-800 mb-1 mt-2">Change on value</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each changeEvents as changeEvent}
					<span class={classNames(badgeClass, colors['blue'])}>{changeEvent}</span>
				{/each}
			</div>
		{/if}
	{/if}
</ScriptSettingsSection>
