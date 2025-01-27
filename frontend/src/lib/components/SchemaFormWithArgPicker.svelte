<script lang="ts">
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import FlowInputEditor from '$lib/components/flows/content/FlowInputEditor.svelte'
	import HistoricInputs from '$lib/components/HistoricInputs.svelte'

	export let schema
	export let args: any
	export let runnableId
	export let runnableType: any

	let rightPanelSize = 40
	let rightHeight = 0

	$: console.log('dbg left panel', rightHeight)

	function openRightPanel() {
		rightPanelSize = 40
	}

	function closeRightPanel() {
		rightPanelSize = 0
	}
</script>

<div class="h-fit">
	<Splitpanes>
		<Pane class="relative">
			<div class="absolute">
				<button on:click={() => {   toogleRightPanel()}> Open </button>
			</div>
			<div class="min-h-[40vh] h-fit" bind:clientHeight={rightHeight}>
				<SchemaForm noVariablePicker compact class="py-4 max-w-3xl" {schema} bind:args />
			</div>
		</Pane>
		{#if rightPanelSize > 0}
			<Pane bind:size={rightPanelSize} minSize={30}>
				<div style="height: {rightHeight}px">
					<FlowInputEditor title="History">
						<HistoricInputs {runnableId} {runnableType} on:select />
					</FlowInputEditor>
				</div>
			</Pane>
		{/if}
	</Splitpanes>
</div>

<style>
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
