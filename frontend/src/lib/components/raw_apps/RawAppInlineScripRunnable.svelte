<script lang="ts">
	import EmptyInlineScript from '../apps/editor/inlineScriptsPanel/EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from '../apps/editor/inlineScriptsPanel/InlineScriptRunnableByPath.svelte'
	import type { Runnable, StaticAppInput } from '../apps/inputType'
	import ScriptEditor from '$lib/components/ScriptEditor.svelte'

	export let runnable: Runnable
	export let id: string
	export let appPath: string

	async function fork(nrunnable: Runnable) {
		runnable = nrunnable == undefined ? undefined : { ...runnable, ...nrunnable }
	}

	function onPick(o: { runnable: Runnable; fields: Record<string, StaticAppInput> }) {
		runnable =
			o.runnable == undefined
				? undefined
				: {
						...(runnable ?? {}),
						...o.runnable
						// fields: o.fields
				  }
	}
</script>

{#if runnable?.type === 'runnableByName' && runnable.inlineScript}
	{#if runnable.inlineScript.language == 'frontend'}
		<div class="text-sm text-tertiary">Frontend scripts not supported for raw apps</div>
	{:else}
		<ScriptEditor
			noHistory
			noSyncFromGithub
			lang={runnable.inlineScript.language}
			path={appPath + '/' + id}
			fixedOverflowWidgets={false}
			bind:code={runnable.inlineScript.content}
			bind:schema={runnable.inlineScript.schema}
			on:createScriptFromInlineScript
			tag={undefined}
			saveToWorkspace
			on:change
		/>
	{/if}
{:else if runnable?.type == 'runnableByPath'}
	<InlineScriptRunnableByPath
		rawApps
		bind:runnable
		fields={{}}
		on:fork={(e) => fork(e.detail)}
		on:delete
		{id}
	/>
{:else}
	<EmptyInlineScript
		unusedInlineScripts={[]}
		rawApps
		on:pick={(e) => onPick(e.detail)}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'runnableByName',
				inlineScript: e.detail,
				name: runnable?.name ?? 'Background Runnable'
			}
		}}
	/>
{/if}
