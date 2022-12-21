<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faClose, faEdit } from '@fortawesome/free-solid-svg-icons'
	import type { ResultAppInput } from '../../inputType'
	import { clearResultAppInput } from '../../utils'
	import InlineScriptEditorDrawer from '../inlineScriptsPanel/InlineScriptEditorDrawer.svelte'

	export let appInput: ResultAppInput
	let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	function edit() {
		if (appInput.runnable?.type === 'runnableByName') {
			inlineScriptEditorDrawer?.openDrawer()
		}
	}

	function clear() {
		appInput = clearResultAppInput(appInput)
	}
</script>

{#if appInput.runnable && appInput.runnable.type === 'runnableByName' && appInput.runnable.inlineScript}
	<InlineScriptEditorDrawer
		bind:this={inlineScriptEditorDrawer}
		bind:inlineScript={appInput.runnable.inlineScript}
	/>
{/if}
<div class="flex justify-between w-full items-center gap-1">
	<span class="text-xs font-semibold">
		{#if appInput.runnable?.type === 'runnableByName'}
			{appInput.runnable.name}
		{:else if appInput.runnable?.type === 'runnableByPath'}
			{appInput.runnable.path}
		{/if}
	</span>
	<div class="flex flex-wrap gap-1">
		{#if appInput.runnable?.type === 'runnableByName' && appInput.runnable.inlineScript}
			<Button size="xs" color="light" variant="border" startIcon={{ icon: faEdit }} on:click={edit}>
				Edit
			</Button>
		{/if}
		<Button size="xs" color="red" variant="border" startIcon={{ icon: faClose }} on:click={clear}>
			Clear
		</Button>
	</div>
</div>
{#if appInput.runnable?.type === 'runnableByName' && !appInput.runnable.inlineScript}
	<span class="text-xs text-gray-500">
		Please configure the language in the inline script panel
	</span>
{/if}
