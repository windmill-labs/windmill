<script lang="ts">
	import { goto } from '$app/navigation'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Fuse from 'fuse.js'

	import { loadHubFlows, sendUserToast } from '$lib/utils'
	import { Button, ButtonPopup, ButtonPopupItem } from '$lib/components/common'
	import ItemPicker from '../ItemPicker.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { flowStore, initFlow } from '$lib/components/flows/flowStore'

	let hubFlows: any[] | undefined = undefined

	const drawers: {
		hub: ItemPicker | undefined
		json: Drawer | undefined
	} = {
		hub: undefined,
		json: undefined
	}
	let hubItems: any[] = []
	let pendingJson: string
	let hubFilter = ''

	const flowHubFuse: Fuse<any> = new Fuse([], {
		includeScore: false,
		keys: ['summary']
	})

	$: filteredHubFlows =
		hubFilter.length > 0 ? flowHubFuse.search(hubFilter).map((value) => value.item) : hubFlows ?? []

	async function importJson() {
		await goto('/flows/add')
		Object.assign($flowStore, JSON.parse(pendingJson))

		initFlow($flowStore)
		drawers.json?.closeDrawer()
	}

	async function loadHubFlowsWFuse(): Promise<void> {
		hubFlows = await loadHubFlows()
		flowHubFuse.setCollection(hubFlows ?? [])
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<ButtonPopup size="md" startIcon={{ icon: faPlus }} href="/flows/add">
		<svelte:fragment slot="main">New Flow</svelte:fragment>
		<ButtonPopupItem on:click={() => drawers.hub?.openDrawer()}>
			Import flow from WindmillHub
		</ButtonPopupItem>
		<ButtonPopupItem on:click={() => drawers.json?.toggleDrawer()}>
			Import flow from raw JSON
		</ButtonPopupItem>
	</ButtonPopup>
</div>

<!-- WindmillHub script list -->
<ItemPicker
	bind:this={drawers.hub}
	pickCallback={(path) => {
		goto('/flows/add?hub=' + path)
	}}
	itemName={'Flow'}
	extraField="summary"
	loadItems={async () => {
		await loadHubFlowsWFuse()
		return filteredHubFlows.map((x) => ({ ...x, path: x.flow_id }))
	}}
/>

<!-- Raw JSON -->
<Drawer bind:this={drawers.json} size="800px">
	<DrawerContent title="Import flow from JSON" on:close={() => drawers.json?.toggleDrawer()}>
		<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" />
		<span slot="submission"><Button size="sm" on:click={importJson}>Import</Button></span>
	</DrawerContent>
</Drawer>
