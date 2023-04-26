<script lang="ts">
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import type { HiddenInlineScript } from '$lib/components/apps/types'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import BackgroundScriptTriggerBy from './shared/BackgroundScriptTriggerBy.svelte'

	export let script: HiddenInlineScript
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
