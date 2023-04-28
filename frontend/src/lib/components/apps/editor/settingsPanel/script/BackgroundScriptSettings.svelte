<script lang="ts">
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import type { AppViewerContext, HiddenInlineScript } from '$lib/components/apps/types'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import BackgroundScriptTriggerBy from './shared/BackgroundScriptTriggerBy.svelte'
	import { getContext } from 'svelte'

	export let script: HiddenInlineScript
	export let id: string

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
</script>

<div class={'border-y border-gray-200 divide-y'}>
	<ScriptSettingHeader name={script.name} />
	<ScriptRunConfiguration
		bind:autoRefresh={script.autoRefresh}
		bind:recomputeOnInputChanged={script.recomputeOnInputChanged}
		canConfigureRecomputeOnInputChanged={script.inlineScript?.language !== 'frontend'}
		on:updateAutoRefresh={() => {
			const autoRefresh = !script.autoRefresh
			if ($runnableComponents?.[id]?.autoRefresh !== autoRefresh && autoRefresh !== undefined) {
				$runnableComponents[id] = {
					...$runnableComponents[id],
					autoRefresh
				}
			}
		}}
	/>
	<BackgroundScriptTriggerBy bind:script />
</div>
