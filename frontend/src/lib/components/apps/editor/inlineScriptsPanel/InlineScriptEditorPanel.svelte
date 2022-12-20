<script lang="ts">
	import type { ResultAppInput } from '../../inputType'
	import EmptyInlineScript from './EmptyInlineScript.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'

	export let componentInput: ResultAppInput
</script>

{#if componentInput.runnable?.type === 'runnableByName' && componentInput.runnable.name}
	{#if componentInput.runnable.inlineScript}
		<InlineScriptEditor
			bind:inlineScript={componentInput.runnable.inlineScript}
			bind:name={componentInput.runnable.name}
			on:delete={() => {
				if (componentInput?.runnable?.type === 'runnableByName') {
					componentInput.runnable = undefined
				}
			}}
		/>
	{:else}
		<EmptyInlineScript
			name={componentInput.runnable.name}
			on:new={(e) => {
				if (componentInput?.runnable?.type === 'runnableByName') {
					componentInput.runnable.inlineScript = e.detail
				}
			}}
		/>
	{/if}
{/if}
