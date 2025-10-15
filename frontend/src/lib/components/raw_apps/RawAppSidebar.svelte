<script lang="ts">
	import PanelSection from '../apps/editor/settingsPanel/common/PanelSection.svelte'
	import type { Runnable } from '../apps/inputType'
	import RawAppInlineScriptPanelList from './RawAppInlineScriptPanelList.svelte'

	interface Props {
		runnables: Record<string, Runnable>
		selectedRunnable: string | undefined
		files: Record<string, string> | undefined
		modules?: Record<string, boolean>
	}

	let { runnables, selectedRunnable = $bindable(), files = $bindable(), modules }: Props = $props()
</script>

<PanelSection fullHeight={false} title="Frontend" id="app-editor-frontend-panel">
	{#each Object.keys(files ?? {}) as file}
		<div class="whitespace-nowrap">
			{file}
		</div>
	{/each}
	<div class="whitespace-nowrap">wmill.ts</div>
	<h4 class="mt-8">Installed modules</h4>
	{#each Object.keys(modules ?? {}) as mod}
		<div class="text-xs">{mod}</div>
	{/each}
</PanelSection>

<RawAppInlineScriptPanelList bind:selectedRunnable {runnables} />
