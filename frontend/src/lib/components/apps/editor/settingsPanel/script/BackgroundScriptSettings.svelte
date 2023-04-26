<script lang="ts">
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import type { AppViewerContext, HiddenInlineScript } from '$lib/components/apps/types'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import BackgroundScriptTriggerBy from './shared/BackgroundScriptTriggerBy.svelte'
	import { getContext } from 'svelte'

	export let script: HiddenInlineScript
	export let id: string

	const { runnableComponents } = getContext<AppViewerContext>('AppViewerContext')

	// If the script's autoRefresh value is different from the one in $runnableComponents, update $runnableComponents
	$: if (
		$runnableComponents?.[id]?.autoRefresh !== script.autoRefresh &&
		script.autoRefresh !== undefined
	) {
		$runnableComponents[id] = {
			...$runnableComponents[id],
			autoRefresh: script.autoRefresh
		}
	}
</script>

<div class={'border-y border-gray-200 divide-y'}>
	<ScriptSettingHeader
		name={script.name}
		badgeLabel={script.inlineScript?.language === 'frontend' ? 'Frontend' : 'Backend'}
	/>
	<ScriptRunConfiguration
		bind:autoRefresh={script.autoRefresh}
		bind:recomputeOnInputChanged={script.recomputeOnInputChanged}
		canConfigureRecomputeOnInputChanged={script.inlineScript?.language !== 'frontend'}
	/>
	<BackgroundScriptTriggerBy bind:script recomputeOnInputChanged={script.recomputeOnInputChanged} />
</div>
