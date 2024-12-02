<script lang="ts">
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import SplitPanesWrapper from './SplitPanesWrapper.svelte'

	export let leftPaneSize = 30
	export let leftPaneMinSize = 25
	export let rightPaneSize = 70
	export let rightPaneMinSize = 25
	export let rightPaneIsFirstInCol = false

	let clientWidth = window.innerWidth
</script>

<main class="h-screen w-full" bind:clientWidth>
	{#if clientWidth >= 768}
		<SplitPanesWrapper class="hidden md:block">
			<Splitpanes>
				<Pane size={30} minSize={25}>
					<slot name="left-pane" />
				</Pane>
				<Pane size={70} minSize={25}>
					<slot name="right-pane" />
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>
	{:else}
		<div class="flex flex-col">
			{#if rightPaneIsFirstInCol}
				<slot name="right-pane" />
				<slot name="left-pane" />
			{:else}
				<slot name="left-pane" />
				<slot name="right-pane" />
			{/if}
		</div>
	{/if}
</main>
