<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import RawAppInlineScripRunnable, { type Runnable } from './RawAppInlineScriptRunnable.svelte'
	import { createScriptFromInlineScript } from '../apps/editor/inlineScriptsPanel/utils'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		appPath: string
		/** Called when code is selected in the editor */
		onSelectionChange?: (
			selection: {
				content: string
				startLine: number
				endLine: number
				startColumn: number
				endColumn: number
			} | null
		) => void
	}

	let {
		runnables = $bindable(),
		selectedRunnable = $bindable(),
		appPath,
		onSelectionChange
	}: Props = $props()
</script>

{#if !selectedRunnable}
	<div class="text-sm text-secondary text-center py-8 px-2">
		Select a runnable on the left panel
	</div>
{:else if runnables?.[selectedRunnable]}
	{#key selectedRunnable}
		<RawAppInlineScripRunnable
			{appPath}
			on:createScriptFromInlineScript={(e) => {
				createScriptFromInlineScript(
					selectedRunnable ?? '',
					e.detail,
					$workspaceStore ?? '',
					appPath
				)
			}}
			on:delete={() => {
				if (selectedRunnable) {
					delete runnables[selectedRunnable]
				}
				selectedRunnable = undefined
			}}
			id={selectedRunnable}
			bind:runnable={runnables[selectedRunnable]}
			{onSelectionChange}
		/>{/key}
{:else}
	<div class="text-sm text-primary text-center py-8 px-2">
		No runnable at id {selectedRunnable}
	</div>
{/if}
