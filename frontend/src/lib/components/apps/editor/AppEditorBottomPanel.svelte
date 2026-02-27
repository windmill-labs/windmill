<script lang="ts">
	import InlineScriptsPanel from './inlineScriptsPanel/InlineScriptsPanel.svelte'
	import RunnableJobPanel from './RunnableJobPanel.svelte'

	interface Props {
		rightPanelSize?: number
		centerPanelWidth?: number
		runnablePanelSize?: number
		onmouseenter?: (...args: any[]) => any
		onmouseleave?: (...args: any[]) => any
		onhidePanel?: (...args: any[]) => any
	}

	let {
		rightPanelSize = 0,
		centerPanelWidth = 0,
		runnablePanelSize = 0,
		onmouseenter = undefined,
		onmouseleave = undefined,
		onhidePanel = undefined
	}: Props = $props()
</script>

{#if rightPanelSize !== 0}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="relative h-full w-full overflow-x-visible"
		onmouseenter={(e) => onmouseenter?.(e)}
		onmouseleave={(e) => onmouseleave?.(e)}
	>
		<InlineScriptsPanel onhidePanel={onhidePanel} />
		<RunnableJobPanel hidden={runnablePanelSize === 0} />
	</div>
{:else}
	<div class="flex flex-row relative w-full h-full">
		<InlineScriptsPanel width={centerPanelWidth * 0.66} onhidePanel={onhidePanel} />
		<RunnableJobPanel float={false} hidden={runnablePanelSize === 0} />
	</div>
{/if}
