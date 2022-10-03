<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'

	import Menu from '$lib/components/common/menu/Menu.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import FlowViewer from '$lib/components/FlowViewer.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { sendUserToast } from '$lib/utils'
	import { faFileExport, faFileImport, faGlobe } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import { Button } from '../../common'
	import { flowStore, initFlow } from '../flowStore'
	import { cleanInputs } from '../utils'

	let jsonSetterDrawer: Drawer
	let jsonViewerDrawer: Drawer
	let pendingJson: string

	function importJson() {
		Object.assign($flowStore, JSON.parse(pendingJson))

		initFlow($flowStore)
		sendUserToast('OpenFlow imported from JSON')
		jsonSetterDrawer.toggleDrawer()
	}
</script>

<Menu placement="bottom-end">
	<button
		slot="trigger"
		class="flex items-center text-sm border focus:outline-none focus:ring-4 font-medium rounded-md text-gray-800 bg-white hover:bg-gray-100 focus:ring-gray-300 px-4 py-2"
	>
		Import/Export
	</button>
	<div class="divide-y divide-gray-200">
		<MenuItem on:click={() => jsonSetterDrawer.toggleDrawer()}>
			<Icon data={faFileImport} scale={0.6} class="inline mr-2" />
			Import from a JSON OpenFlow
		</MenuItem>
		<MenuItem on:click={() => jsonViewerDrawer.toggleDrawer()}>
			<Icon data={faFileExport} scale={0.6} class="inline mr-2" />
			Export to a JSON OpenFlow
		</MenuItem>
	</div>
</Menu>

<Drawer bind:this={jsonSetterDrawer} size="800px">
	<DrawerContent title="Import JSON" on:close={() => jsonSetterDrawer.toggleDrawer()}>
		<Button size="sm" on:click={importJson}>Import</Button>
		<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" />
	</DrawerContent>
</Drawer>
<Drawer bind:this={jsonViewerDrawer} size="800px">
	<DrawerContent title="See JSON" on:close={() => jsonViewerDrawer.toggleDrawer()}>
		{#if $flowStore}
			<FlowViewer flow={cleanInputs($flowStore)} tab="json" />
		{/if}
	</DrawerContent>
</Drawer>
