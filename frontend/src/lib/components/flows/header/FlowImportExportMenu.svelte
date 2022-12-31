<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import FlowViewer from '$lib/components/FlowViewer.svelte'

	import { faFileExport } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Button } from '../../common'
	import { flowStore } from '../flowStore'
	import { cleanInputs } from '../utils'

	let jsonViewerDrawer: Drawer
</script>

<div class="flex-row gap-x-2 hidden sm:flex">
	<Button
		btnClasses="mr-2"
		size="sm"
		variant="border"
		color="light"
		on:click={() => jsonViewerDrawer.toggleDrawer()}
	>
		<Icon data={faFileExport} scale={0.6} class="inline mr-2" />
		Export JSON
	</Button>
</div>

<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="OpenFlow JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		{#if $flowStore}
			<FlowViewer flow={cleanInputs($flowStore)} tab="json" />
		{/if}
	</DrawerContent>
</Drawer>
