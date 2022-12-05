<script lang="ts">
	import { goto } from '$app/navigation'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'

	import { Button, ButtonPopup, ButtonPopupItem } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { flowStore, initFlow } from '$lib/components/flows/flowStore'

	let drawer: Drawer | undefined = undefined
	let pendingJson: string

	async function importJson() {
		await goto('/flows/add')
		Object.assign($flowStore, JSON.parse(pendingJson))

		initFlow($flowStore)
		drawer?.closeDrawer?.()
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<ButtonPopup size="sm" spacingSize="xl" startIcon={{ icon: faPlus }} href="/flows/add">
		<svelte:fragment slot="main">New Flow</svelte:fragment>
		<ButtonPopupItem on:click={() => drawer?.toggleDrawer?.()}>
			Import from raw JSON
		</ButtonPopupItem>
	</ButtonPopup>
</div>

<!-- Raw JSON -->
<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Import flow from JSON" on:close={() => drawer?.toggleDrawer?.()}>
		<SimpleEditor bind:code={pendingJson} lang="json" class="h-full" fixedOverflowWidgets={false} />
		<svelte:fragment slot="actions">
			<Button size="sm" on:click={importJson}>Import</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
