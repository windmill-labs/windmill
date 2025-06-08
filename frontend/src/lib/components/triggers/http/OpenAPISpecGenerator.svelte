<script lang="ts">
	import { DrawerContent, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { Save } from 'lucide-svelte'

	let openAPIGenerator: Drawer

	export function openDrawer() {
		openAPIGenerator.openDrawer()
	}
</script>

<Drawer size="700px" bind:this={openAPIGenerator}>
	<DrawerContent
		title={'Generate HTTP routes from OpenAPI spec'}
		on:close={() => openAPIGenerator.closeDrawer()}
	>
		<svelte:fragment slot="actions">
			<Button startIcon={{ icon: Save }}>Save routes</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
