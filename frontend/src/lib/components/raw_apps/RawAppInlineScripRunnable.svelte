<script lang="ts">
	import EmptyInlineScript from '../apps/editor/inlineScriptsPanel/EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from '../apps/editor/inlineScriptsPanel/InlineScriptRunnableByPath.svelte'
	import type { RunnableWithFields, StaticAppInput } from '../apps/inputType'
	import { createEventDispatcher } from 'svelte'
	import RawAppInlineScriptEditor from './RawAppInlineScriptEditor.svelte'

	export let runnable: RunnableWithFields | undefined
	export let id: string
	export let appPath: string

	const dispatch = createEventDispatcher()

	async function fork(nrunnable: RunnableWithFields) {
		runnable = nrunnable == undefined ? undefined : { ...runnable, ...nrunnable }
	}

	function onPick(o: { runnable: RunnableWithFields; fields: Record<string, StaticAppInput> }) {
		runnable =
			o.runnable == undefined
				? undefined
				: {
						...(runnable ?? {}),
						...o.runnable,
						fields: o.fields
				  }
	}
</script>

{#if runnable?.type === 'runnableByName' && runnable.inlineScript}
	{#if runnable.inlineScript.language == 'frontend'}
		<div class="text-sm text-tertiary">Frontend scripts not supported for raw apps</div>
	{:else}
		<RawAppInlineScriptEditor
			on:createScriptFromInlineScript={() => dispatch('createScriptFromInlineScript', runnable)}
			{id}
			bind:inlineScript={runnable.inlineScript}
			bind:name={runnable.name}
			bind:fields={runnable.fields}
			on:delete
			path={appPath}
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
