<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import RawAppInlineScripRunnable from './RawAppInlineScriptRunnable.svelte'
	import { createScriptFromInlineScript } from '../apps/editor/inlineScriptsPanel/utils'
	import type { Runnable } from '../apps/inputType'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		appPath: string
	}

	let { runnables, selectedRunnable = $bindable(), appPath }: Props = $props()
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
		/>{/key}
{:else}
	<div class="text-sm text-tertiary text-center py-8 px-2">
		No runnable at id {selectedRunnable}
	</div>
{/if}
