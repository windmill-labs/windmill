<script lang="ts" context="module">
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
	import { isTriggerable } from './utils'

	import type { AppComponent } from '../../component'
	import ScriptTransformer from './shared/ScriptTransformer.svelte'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import ComponentScriptTriggerBy from './shared/ComponentScriptTriggerBy.svelte'
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import ScriptSettingsSection from './shared/ScriptSettingsSection.svelte'

	export let appInput: ResultAppInput
	export let appComponent: AppComponent
	export let hasScript: boolean

	let runnable = appInput.runnable

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	export let actions: ActionType[] = []
</script>

<div class={'border border-gray-200 divide-y'}>
	<ScriptSettingHeader
		name={runnable?.type === 'runnableByName'
			? runnable.name
			: runnable?.type === 'runnableByPath'
			? runnable.path
			: ''}
		{actions}
	/>

	{#if hasScript}
		<ScriptTransformer bind:appInput bind:appComponent />
		<ScriptRunConfiguration
			canConfigureRecomputeOnInputChanged={!isTriggerable(appComponent.type)}
			canConfigureRunOnStart={!isTriggerable(appComponent.type)}
			bind:autoRefresh={appInput.autoRefresh}
			bind:recomputeOnInputChanged={appInput.recomputeOnInputChanged}
			on:updateAutoRefresh={() => {
				const autoRefresh = !(
					appComponent.componentInput?.type === 'runnable' &&
					appComponent?.componentInput?.autoRefresh
				)

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
			}}
		/>
		<ComponentScriptTriggerBy {appComponent} {appInput} />
	{:else}
		<ScriptSettingsSection title="Language selection">
			<div class="text-xs"> Please configure the language in the inline script panel </div>
		</ScriptSettingsSection>
	{/if}
</div>
