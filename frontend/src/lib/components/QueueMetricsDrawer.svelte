<script lang="ts">
	import { Drawer, DrawerContent } from './common'
	import QueueMetricsDrawerInner from './QueueMetricsDrawerInner.svelte'
	import QueueAlerts from './QueueAlerts.svelte'

	let drawer: Drawer
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

		<div class="py-8"></div>
	</DrawerContent>
</Drawer>
