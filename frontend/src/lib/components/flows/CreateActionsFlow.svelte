<script lang="ts">
	import { goto } from '$app/navigation'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'

	import { Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import SimpleEditor from '$lib/components/SimpleEditor.svelte'
	import { importFlowStore } from '$lib/components/flows/flowStore'
	import { Plus } from 'lucide-svelte'

	let drawer: Drawer | undefined = undefined
	let pendingJson: string

	async function importJson() {
		$importFlowStore = JSON.parse(pendingJson)
		await goto('/flows/add')
		drawer?.closeDrawer?.()
	}
</script>

<!-- Buttons -->
<div class="flex flex-row gap-2">
	<Button
		size="sm"
		spacingSize="xl"
		startIcon={{ icon: Plus }}
		endIcon={{ icon: BarsStaggered }}
		href="/flows/add?nodraft=true"
		dropdownItems={[
			{
				label: 'Import from JSON',
				onClick: () => drawer?.toggleDrawer?.()
			}
		]}
	>
		Flow
	</Button>
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
