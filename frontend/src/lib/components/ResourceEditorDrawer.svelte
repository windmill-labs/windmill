<script lang="ts">
	import ResourceEditor from './ResourceEditor.svelte'
	import { Button, Drawer } from './common'

	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { Save } from 'lucide-svelte'

	let drawer: Drawer
	let canSave = true
	let resource_type: string | undefined = undefined

	let resourceEditor: ResourceEditor | undefined = undefined

	let path: string | undefined = undefined

	let newResource = false
	export async function initEdit(p: string): Promise<void> {
		resource_type = undefined
		newResource = false
		path = p
		drawer.openDrawer?.()
	}

	export async function initNew(resourceType: string): Promise<void> {
		newResource = true
		path = undefined
		resource_type = resourceType
		drawer.openDrawer?.()
	}
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title={path ? 'Edit ' + path : 'Add a resource'} on:close={drawer.closeDrawer}>
		<ResourceEditor
			{newResource}
			{path}
			{resource_type}
			on:refresh
			bind:this={resourceEditor}
			bind:canSave
		/>
		<svelte:fragment slot="actions">
			<Button
				startIcon={{ icon: Save }}
				on:click={() => {
					resourceEditor?.editResource()
					drawer.closeDrawer()
				}}
				disabled={!canSave}
			>
				Save
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
