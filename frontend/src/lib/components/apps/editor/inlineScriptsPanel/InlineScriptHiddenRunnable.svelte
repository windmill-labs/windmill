<script lang="ts">
	import type { HiddenRunnable } from '../../types'
	import InlineScriptEditor from './InlineScriptEditor.svelte'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptRunnableByPath from './InlineScriptRunnableByPath.svelte'
	import type { Runnable, StaticAppInput } from '../../inputType'

	export let runnable: HiddenRunnable
	export let id: string

	async function fork(nrunnable: Runnable) {
		runnable = { ...runnable, ...nrunnable }
	}
	function onPick(o: { runnable: Runnable; fields: Record<string, StaticAppInput> }) {
		runnable = {
			...runnable,
			...o.runnable,
			fields: o.fields
		}
	}
</script>

{#if runnable?.type === 'runnableByName' && runnable.inlineScript}
	<InlineScriptEditor
		{id}
		bind:inlineScript={runnable.inlineScript}
		bind:name={runnable.name}
		bind:fields={runnable.fields}
		syncFields
		on:delete
	/>
{:else if runnable?.type == 'runnableByPath'}
	<InlineScriptRunnableByPath
		bind:runnable
		bind:fields={runnable.fields}
		on:fork={(e) => fork(e.detail)}
	/>
{:else}
	<EmptyInlineScript
		on:pick={(e) => onPick(e.detail)}
		name={runnable.name}
		on:delete
		showScriptPicker
		on:new={(e) => {
			runnable = {
				type: 'runnableByName',
				inlineScript: e.detail,
				name: runnable.name,
				fields: {}
			}
		}}
	/>
{/if}
