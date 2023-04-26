<script lang="ts">
	import type { ResultAppInput } from '$lib/components/apps/inputType'
	import type { ButtonType } from '$lib/components/common/button/model'
	import { isFrontend, isTriggerable } from './utils'

	import type { AppComponent } from '../../component'
	import ScriptTransformer from './shared/ScriptTransformer.svelte'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import ComponentScriptTriggerBy from './shared/ComponentScriptTriggerBy.svelte'
	import ScriptSettingsActions from './shared/ScriptSettingsActions.svelte'
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'

	type ActionType = {
		label: string
		icon: any
		color: ButtonType.Color
		callback: () => void
	}

	export let appInput: ResultAppInput
	export let appComponent: AppComponent

	let runnable = appInput.runnable

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	export let actions: ActionType[] = []

	$: if (
		appComponent.componentInput?.type === 'runnable' &&
		$runnableComponents?.[appComponent.id]?.autoRefresh !==
			appComponent.componentInput.autoRefresh &&
		appComponent.componentInput.autoRefresh !== undefined
	) {
		$runnableComponents[appComponent.id] = {
			...$runnableComponents[appComponent.id],
			autoRefresh: appComponent.componentInput.autoRefresh
		}
	}
</script>

<ScriptSettingsActions {actions} />
<div class={'border border-gray-200 divide-y'}>
	<ScriptSettingHeader
		name={runnable?.type === 'runnableByName'
			? runnable.name
			: runnable?.type === 'runnableByPath'
			? runnable.path
			: ''}
		badgeLabel={isFrontend(runnable) ? 'Frontend' : 'Backend'}
	/>
	<ScriptTransformer bind:appInput bind:appComponent />
	<ScriptRunConfiguration
		bind:autoRefresh={appInput.autoRefresh}
		bind:recomputeOnInputChanged={appInput.recomputeOnInputChanged}
		canConfigureRecomputeOnInputChanged={!isTriggerable(appComponent.type)}
		canConfigureRunOnStart={!isTriggerable(appComponent.type)}
	/>
	<ComponentScriptTriggerBy {appComponent} {appInput} />
</div>
