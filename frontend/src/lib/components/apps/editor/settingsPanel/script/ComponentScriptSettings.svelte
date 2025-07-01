<script lang="ts" module>
	export type ActionType = {
		label: string
		icon: any
		color: ButtonType.Color
		callback: () => void
	}
</script>

<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import type { ButtonType } from '$lib/components/common/button/model'
	import { isTriggerable, isFrontend } from './utils'

	import type { AppComponent } from '../../component'
	import ScriptTransformer from './shared/ScriptTransformer.svelte'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import ComponentScriptTriggerBy from './shared/ComponentScriptTriggerBy.svelte'
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import ScriptSettingsSection from './shared/ScriptSettingsSection.svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	const { runnableComponents, stateId } = getContext<AppViewerContext>('AppViewerContext')
	interface Props {
		appInput: ResultAppInput
		appComponent: AppComponent
		hasScript: boolean
		actions?: ActionType[]
	}

	let {
		appInput = $bindable(),
		appComponent = $bindable(),
		hasScript,
		actions = []
	}: Props = $props()

	function updateAutoRefresh() {
		const autoRefresh =
			appComponent.componentInput?.type === 'runnable' && appComponent?.componentInput?.autoRefresh

		if (
			appComponent.componentInput?.type === 'runnable' &&
			$runnableComponents?.[appComponent.id]?.autoRefresh !== autoRefresh &&
			!isTriggerable(appComponent.type) &&
			autoRefresh !== undefined
		) {
			$runnableComponents[appComponent.id] = {
				...$runnableComponents[appComponent.id],
				autoRefresh
			}
		}
	}
</script>

<div>
	{#key $stateId}
		{@const runnable = appInput.runnable}
		<ScriptSettingHeader
			name={runnable?.type === 'runnableByName'
				? runnable.name
				: runnable?.type === 'runnableByPath'
					? runnable.path
					: ''}
			{actions}
		/>
	{/key}
	{#if !isTriggerable(appComponent.type)}
		<div class="flex items-center justify-between w-full">
			<div class="flex flex-row items-center gap-2 text-xs"> Hide Refresh Button </div>

			<Toggle bind:checked={appInput.hideRefreshButton} size="xs" class="my-2" />
		</div>
	{/if}
	{#if hasScript}
		<ScriptTransformer bind:appInput id={appComponent.id} />
		<ScriptRunConfiguration
			canConfigureRecomputeOnInputChanged={!isTriggerable(appComponent.type) &&
				!isFrontend(appInput.runnable)}
			canConfigureRunOnStart={!isTriggerable(appComponent.type)}
			bind:autoRefresh={appInput.autoRefresh}
			bind:recomputeOnInputChanged={appInput.recomputeOnInputChanged}
			on:updateAutoRefresh={updateAutoRefresh}
		>
			<ComponentScriptTriggerBy {appComponent} {appInput} />
		</ScriptRunConfiguration>
	{:else}
		<ScriptSettingsSection title="Language selection">
			<div class="text-xs"> Please configure the language in the inline script panel </div>
		</ScriptSettingsSection>
	{/if}
</div>
