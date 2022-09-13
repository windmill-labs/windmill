<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'

	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	$: upToDisabled = ['settings', 'inputs', 'schedules'].includes($selectedId)
</script>

<span class=" inline-flex  items-center">
	<span class="text-md mr-2">Preview:</span>
	<button
		disabled={upToDisabled}
		on:click={() => {
			previewMode = 'upTo'

			previewOpen = !previewOpen
		}}
		type="button"
		class="relative inline-flex items-center rounded-l-md border border-gray-300 bg-white px-2 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:z-10 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
	>
		Up to this step
	</button>

	<button
		on:click={() => {
			previewMode = 'whole'
			previewOpen = !previewOpen
		}}
		type="button"
		class="relative -ml-px inline-flex items-center rounded-r-md border border-gray-300 bg-white px-2 py-1 text-sm font-medium text-gray-700 hover:bg-gray-50 focus:z-10 focus:border-indigo-500 focus:outline-none focus:ring-1 focus:ring-indigo-500"
	>
		Flow
	</button>
</span>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent bind:previewMode on:close={() => (previewOpen = !previewOpen)} />
</Drawer>
