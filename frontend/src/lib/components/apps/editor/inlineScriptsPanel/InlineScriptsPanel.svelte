<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import InlineScriptEditor from './InlineScriptEditor.svelte'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let selectedScriptComponentId: string | undefined = undefined
</script>

<SplitPanesWrapper>
	<Pane size={25}>
		<InlineScriptsPanelList bind:selectedScriptComponentId />
	</Pane>
	<Pane size={75}>
		{#each $app.grid as gridComponent, index (index)}
			{#if gridComponent.data.id === selectedScriptComponentId}
				<InlineScriptEditor bind:componentInput={gridComponent.data.componentInput} />
			{/if}
		{/each}
	</Pane>
</SplitPanesWrapper>
