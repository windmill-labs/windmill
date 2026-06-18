<script lang="ts">
	import type { ResultAppInput, Runnable, StaticAppInput } from '$lib/components/apps/inputType'
	import { isScriptByNameDefined, isScriptByPathDefined } from '$lib/components/apps/utils'
	import { getContext } from 'svelte'
	import type { AppComponent } from '../../component'
	import RunnableSelector from '../mainInput/RunnableSelector.svelte'
	import SelectedRunnable from '../SelectedRunnable.svelte'
	import type { AppEditorContext, AppViewerContext } from '$lib/components/apps/types'
	import { convertManagedFieldsToEvalv2 } from '$lib/components/apps/components/componentManagedFields'

	interface Props {
		appInput: ResultAppInput
		defaultUserInput?: boolean
		appComponent: AppComponent
	}

	let { appInput = $bindable(), defaultUserInput = false, appComponent }: Props = $props()

	const { selectedComponentInEditor } = getContext<AppEditorContext>('AppEditorContext')
	const { app } = getContext<AppViewerContext>('AppViewerContext')

	function onPick({
		runnable,
		fields
	}: {
		runnable: Runnable
		fields: Record<string, StaticAppInput>
	}) {
		if (appInput.type === 'runnable') {
			// Convert component-managed fields from static to evalv2 type
			// This ensures they are properly handled at runtime and not added to force_viewer_static_fields
			const convertedFields = convertManagedFieldsToEvalv2(
				appComponent.type,
				appComponent.id,
				fields
			) as Record<string, StaticAppInput>
			appInput = { ...appInput, runnable, fields: convertedFields }
			$selectedComponentInEditor = appComponent.id
		} else {
			console.warn('Cannot pick runnable for non-runnable input')
		}
	}
</script>

{#if isScriptByPathDefined(appInput) || isScriptByNameDefined(appInput)}
	<SelectedRunnable {appComponent} bind:appInput />
{:else if appInput !== undefined}
	<RunnableSelector
		unusedInlineScripts={$app.unusedInlineScripts}
		hideCreateScript={appComponent.type === 'flowstatuscomponent'}
		onlyFlow={appComponent.type === 'flowstatuscomponent'}
		{defaultUserInput}
		on:pick={(e) => onPick(e.detail)}
	/>
{/if}
