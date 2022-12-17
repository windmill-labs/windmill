<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import { faClose, faEdit } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import type { ResultAppInput } from '../../inputType'
	import type InlineScriptEditorDrawer from '../inlineScriptsPanel/InlineScriptEditorDrawer.svelte'

	export let appInput: ResultAppInput

	const dispatch = createEventDispatcher()
	export let inlineScriptEditorDrawer: InlineScriptEditorDrawer

	function edit() {
		if (appInput.runnable?.type === 'runnableByName') {
			inlineScriptEditorDrawer.openDrawer(appInput.runnable.inlineScriptName)
		}
	}

	function clear() {
		dispatch('clear')
	}
</script>

{#if appInput.runnable}
	<div class="flex justify-between w-full items-center">
		<span class="text-xs font-semibold">
			{appInput.runnable.type === 'runnableByName'
				? appInput.runnable.inlineScriptName
				: appInput.runnable.path}
		</span>
		<div>
			<Button size="xs" color="light" variant="border" startIcon={{ icon: faEdit }} on:click={edit}>
				Edit
			</Button>
			<Button size="xs" color="red" variant="border" startIcon={{ icon: faClose }} on:click={clear}>
				Clear
			</Button>
		</div>
	</div>
{/if}
