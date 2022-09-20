<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { faPlay } from '@fortawesome/free-solid-svg-icons'

	import { getContext } from 'svelte'
	import Icon from 'svelte-awesome'
	import type { FlowEditorContext } from '../types'
	const { selectedId } = getContext<FlowEditorContext>('FlowEditorContext')
	let previewOpen = false
	let previewMode: 'upTo' | 'whole' = 'whole'

	$: upToDisabled = [
		'settings',
		'settings-schedule',
		'settings-retries',
		'inputs',
		'schedules'
	].includes($selectedId)
</script>

<span class="space-x-2 flex h-8">
	<button
		type="button"
		disabled={upToDisabled}
		on:click={() => {
			previewMode = 'upTo'

			previewOpen = !previewOpen
		}}
		class="flex items-center text-gray-900 bg-white border border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-lg text-sm px-4 py-2"
	>
		Test up to this step
		<Icon data={faPlay} class="ml-2" scale={0.8} />
	</button>

	<button
		type="button"
		on:click={() => {
			previewMode = 'whole'
			previewOpen = !previewOpen
		}}
		class="flex items-center  text-white bg-blue-500 hover:bg-blue-700 focus:ring-4 focus:ring-blue-300 font-medium rounded-lg text-sm px-4 py-1 focus:outline-none "
	>
		Test flow
		<Icon data={faPlay} class="ml-2" scale={0.8} />
	</button>
</span>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent bind:previewMode on:close={() => (previewOpen = !previewOpen)} />
</Drawer>
