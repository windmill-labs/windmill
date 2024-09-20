<script lang="ts">
	import { Drawer, DrawerContent } from './common'

	import QueueMetricsDrawerInner from './QueueMetricsDrawerInner.svelte'
	let drawer: Drawer
	export function openDrawer() {
		drawer?.openDrawer()
	}
</script>

<Drawer bind:this={drawer}>
	<DrawerContent title="Queue Metrics" on:close={drawer.closeDrawer}>
		<QueueMetricsDrawerInner />
	</DrawerContent>
</Drawer>
