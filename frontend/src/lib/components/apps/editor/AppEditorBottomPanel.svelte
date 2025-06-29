<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'
	import RunnableJobPanel from './RunnableJobPanel.svelte'

	interface Props {
		rightPanelSize?: number
		centerPanelWidth?: number
		runnablePanelSize?: number
	}

	let { rightPanelSize = 0, centerPanelWidth = 0, runnablePanelSize = 0 }: Props = $props()
</script>

{#if rightPanelSize !== 0}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="relative h-full w-full overflow-x-visible"
		onmouseenter={bubble('mouseenter')}
		onmouseleave={bubble('mouseleave')}
	>
		<InlineScriptsPanel on:hidePanel />
		<RunnableJobPanel hidden={runnablePanelSize === 0} />
	</div>
{:else}
	<div class="flex flex-row relative w-full h-full">
		<InlineScriptsPanel width={centerPanelWidth * 0.66} on:hidePanel />
		<RunnableJobPanel float={false} hidden={runnablePanelSize === 0} />
	</div>
{/if}
