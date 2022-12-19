<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane } from 'svelte-splitpanes'
	import InlineScriptsPanelList from './InlineScriptsPanelList.svelte'
	import Xyz from './Xyz.svelte'

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let selectedScriptName: string | undefined = undefined
</script>

<SplitPanesWrapper>
	<Pane size={25}>
		<InlineScriptsPanelList bind:selectedScriptName />
	</Pane>
	<Pane size={75}>
		{#each $app.grid as gridComponent, index}
			<Xyz bind:componentInput={gridComponent.data.componentInput} {selectedScriptName} />
		{/each}
	</Pane>
</SplitPanesWrapper>
