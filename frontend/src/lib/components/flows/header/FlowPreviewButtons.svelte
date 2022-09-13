<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false

	$: upToDisabled = ['settings', 'inputs', 'schedules'].includes($selectedId)
</script>

<div class="flex space-x-2">
	<button
		disabled={upToDisabled}
		on:click={() => {
			previewOpen = !previewOpen
		}}
		type="button"
		class="py-2 px-3 text-xs font-medium text-center text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
	>
		Up to this step
	</button>
	<button
		type="button"
		on:click={() => {
			previewOpen = !previewOpen
		}}
		class="py-2 px-3 text-xs font-medium text-center text-white bg-blue-700 rounded-lg hover:bg-blue-800 focus:ring-4 focus:outline-none focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800"
	>
		Preview whole
	</button>
</div>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent args={{}} on:close={() => (previewOpen = !previewOpen)} />
</Drawer>
