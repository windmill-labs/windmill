<script lang="ts">
	import { flowStore } from '$lib/components/flows/flowStore'
	import FlowPreviewButtons from './FlowPreviewButtons.svelte'
	import FlowStatus from './FlowStatus.svelte'
</script>

<div class="flex justify-between items-center border-y p-2">
	<div id="flow_title">
		{$flowStore.path}
	</div>
	<FlowStatus />
	<FlowPreviewButtons />
</div>
