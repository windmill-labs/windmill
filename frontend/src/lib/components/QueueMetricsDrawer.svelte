<script lang="ts">
	import { Drawer, DrawerContent } from './common'
	import QueueMetricsDrawerInner from './QueueMetricsDrawerInner.svelte'
	import QueueAlerts from './QueueAlerts.svelte'
	import WorkspaceFairnessEvents from './WorkspaceFairnessEvents.svelte'
	import { isCloudHosted } from '$lib/cloud'

	let drawer: Drawer | undefined = $state()
	export function openDrawer() {
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer} size="1000px">
	<DrawerContent
		title="Queues"
		on:close={drawer.closeDrawer}
		documentationLink="https://www.windmill.dev/docs/core_concepts/worker_groups#queue-metrics"
	>
		<QueueAlerts />

		<div class="py-8"></div>

		<QueueMetricsDrawerInner />

		{#if isCloudHosted()}
			<div class="py-8"></div>
			<WorkspaceFairnessEvents />
		{/if}

		<div class="py-8"></div>
	</DrawerContent>
</Drawer>
