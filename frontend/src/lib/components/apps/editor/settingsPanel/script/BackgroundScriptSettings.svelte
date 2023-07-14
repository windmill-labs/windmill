<script lang="ts">
	import ScriptSettingHeader from './shared/ScriptSettingHeader.svelte'
	import type { AppViewerContext, HiddenRunnable } from '$lib/components/apps/types'
	import ScriptRunConfiguration from './shared/ScriptRunConfiguration.svelte'
	import BackgroundScriptTriggerBy from './shared/BackgroundScriptTriggerBy.svelte'
	import { getContext } from 'svelte'
	import ScriptSettingsSection from './shared/ScriptSettingsSection.svelte'
	import ScriptTransformer from './shared/ScriptTransformer.svelte'

	export let runnable: HiddenRunnable
	export let id: string

	const { runnableComponents, app } = getContext<AppViewerContext>('AppViewerContext')

	function updateAutoRefresh() {
		const autoRefresh = runnable.autoRefresh
		if ($runnableComponents?.[id]?.autoRefresh !== autoRefresh && autoRefresh !== undefined) {
			$runnableComponents[id] = {
				...$runnableComponents[id],
				autoRefresh
			}
		}
	}

	$: runnable && ($app = $app)
</script>

<div class={'border-y border-gray-200 divide-y'}>
	<ScriptSettingHeader name={runnable.name} />
	<ScriptTransformer bind:appInput={runnable} {id} />
	{#if runnable.type == 'runnableByPath' || runnable.inlineScript}
		<ScriptRunConfiguration
			bind:autoRefresh={runnable.autoRefresh}
			bind:recomputeOnInputChanged={runnable.recomputeOnInputChanged}
			canConfigureRecomputeOnInputChanged={runnable.type == 'runnableByPath' ||
				runnable.inlineScript?.language !== 'frontend'}
			on:updateAutoRefresh={updateAutoRefresh}
		>
			<BackgroundScriptTriggerBy
				bind:script={runnable}
				recomputeOnInputChanged={runnable.recomputeOnInputChanged}
			/>
		</ScriptRunConfiguration>
	{:else}
		<ScriptSettingsSection title="Language selection">
			<div class="text-xs"> Please configure the language in the inline script panel </div>
		</ScriptSettingsSection>
	{/if}
</div>
