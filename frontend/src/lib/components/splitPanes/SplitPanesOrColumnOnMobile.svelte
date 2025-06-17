<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SplitPanesWrapper from './SplitPanesWrapper.svelte'

	interface Props {
		leftPaneSize?: number
		leftPaneMinSize?: number
		rightPaneSize?: number
		rightPaneMinSize?: number
		rightPaneIsFirstInCol?: boolean
		left_pane?: import('svelte').Snippet
		right_pane?: import('svelte').Snippet
	}

	let {
		leftPaneSize = 30,
		leftPaneMinSize = 25,
		rightPaneSize = 70,
		rightPaneMinSize = 25,
		rightPaneIsFirstInCol = false,
		left_pane,
		right_pane
	}: Props = $props()

	let clientWidth = $state(window.innerWidth)
</script>

<main class="flex-grow w-full overflow-y-auto" bind:clientWidth>
	{#if clientWidth >= 768}
		<SplitPanesWrapper class="hidden md:block">
			<Splitpanes>
				<Pane size={leftPaneSize} minSize={leftPaneMinSize}>
					{@render left_pane?.()}
				</Pane>
				<Pane size={rightPaneSize} minSize={rightPaneMinSize}>
					{@render right_pane?.()}
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>
	{:else}
		<div class="flex flex-col">
			{#if rightPaneIsFirstInCol}
				{@render right_pane?.()}
				{@render left_pane?.()}
			{:else}
				{@render left_pane?.()}
				{@render right_pane?.()}
			{/if}
		</div>
	{/if}
</main>
