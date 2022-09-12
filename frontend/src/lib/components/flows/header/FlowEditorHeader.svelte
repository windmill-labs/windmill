<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import FlowPreviewContent from '$lib/components/FlowPreviewContent.svelte'
	import { flowStore } from '$lib/components/flows/flowStore'
	import FlowPreviewButtons from './FlowPreviewButtons.svelte'
	import FlowStatus from './FlowStatus.svelte'

	let previewOpen = false
</script>

<Drawer bind:open={previewOpen} size="800px">
	<FlowPreviewContent on:close={() => (previewOpen = !previewOpen)} />
</Drawer>

<div class="flex justify-between items-center border-y p-2">
	<div id="flow_title">
		{$flowStore.path}
	</div>
	<FlowStatus />
	<FlowPreviewButtons />
</div>
