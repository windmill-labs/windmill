<script lang="ts">
	import { Button, Drawer } from './common'

	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { Loader2, Save } from 'lucide-svelte'

	let drawer: Drawer | undefined = $state()
	let canSave = $state(true)
	let resource_type: string | undefined = $state(undefined)
	let defaultValues: Record<string, any> | undefined = $state(undefined)

	let resourceEditor: { editResource: () => void; createResource: () => void } | undefined =
		$state(undefined)

	let path: string | undefined = $state(undefined)

	export async function initEdit(p: string): Promise<void> {
		resource_type = undefined
		path = p
		drawer?.openDrawer?.()
	}

	export async function initNew(
		resourceType: string,
		nDefaultValues?: Record<string, any>
	): Promise<void> {
		path = undefined
		resource_type = resourceType
		defaultValues = nDefaultValues
		drawer?.openDrawer?.()
	}

	let mode: 'edit' | 'new' = $derived(!path ? 'new' : 'edit')
</script>

<Drawer bind:this={drawer} size="800px">
	<DrawerContent
		title={mode == 'edit' ? 'Edit ' + path : 'Add a resource'}
		on:close={drawer?.closeDrawer}
	>
		{#await import('./ResourceEditor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				{path}
				{resource_type}
				{defaultValues}
				on:refresh
				bind:this={resourceEditor}
				bind:canSave
			/>
		{/await}
		{#snippet actions()}
			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				on:click={() => {
					if (mode == 'edit') {
						resourceEditor?.editResource()
					} else {
						resourceEditor?.createResource()
					}
					drawer?.closeDrawer()
				}}
				disabled={!canSave}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
