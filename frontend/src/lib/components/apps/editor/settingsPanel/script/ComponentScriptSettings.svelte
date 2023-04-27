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

	export let appInput: ResultAppInput
	export let appComponent: AppComponent

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
	<ScriptTransformer bind:appInput bind:appComponent />
	<ScriptRunConfiguration
		bind:autoRefresh={appInput.autoRefresh}
		bind:recomputeOnInputChanged={appInput.recomputeOnInputChanged}
		canConfigureRecomputeOnInputChanged={!isTriggerable(appComponent.type)}
		canConfigureRunOnStart={!isTriggerable(appComponent.type)}
		on:updateAutoRefresh={() => {
			if (
				appComponent.componentInput?.type === 'runnable' &&
				$runnableComponents?.[appComponent.id]?.autoRefresh !==
					appComponent.componentInput.autoRefresh &&
				!isTriggerable(appComponent.type) &&
				appComponent.componentInput.autoRefresh !== undefined
			) {
				$runnableComponents[appComponent.id] = {
					...$runnableComponents[appComponent.id],
					autoRefresh: appComponent.componentInput.autoRefresh
				}
			}
		}}
	/>
	<ComponentScriptTriggerBy {appComponent} {appInput} />
</div>
