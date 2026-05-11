<script lang="ts">
	import { Button, Drawer } from './common'

	import DrawerContent from './common/drawer/DrawerContent.svelte'

	import { Loader2, Save } from 'lucide-svelte'
	import WsSpecificVersions from './WsSpecificVersions.svelte'
	import { workspaceStore } from '$lib/stores'

	let {
		workspace = undefined,
		disableChatOffset = false
	}: { workspace?: string; disableChatOffset?: boolean } = $props()

	let drawer: Drawer | undefined = $state()
	let canSave = $state(true)
	let resource_type: string | undefined = $state(undefined)
	let defaultValues: Record<string, any> | undefined = $state(undefined)

	let resourceEditor: { save: () => void } | undefined = $state(undefined)

	let path: string | undefined = $state(undefined)
	let selected: string | undefined = $state(undefined)

	let effectiveWorkspace = $derived(workspace ?? $workspaceStore!)

	export async function initEdit(p: string): Promise<void> {
		resource_type = undefined
		path = p
		selected = effectiveWorkspace
		drawer?.openDrawer?.()
	}

	export async function initNew(
		resourceType: string,
		nDefaultValues?: Record<string, any>
	): Promise<void> {
		path = undefined
		resource_type = resourceType
		defaultValues = nDefaultValues
		selected = effectiveWorkspace
		drawer?.openDrawer?.()
	}

	let mode: 'edit' | 'new' = $derived(!path ? 'new' : 'edit')
</script>

<Drawer bind:this={drawer} size="50rem" {disableChatOffset}>
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
				{workspace}
				on:refresh
				bind:this={resourceEditor}
				bind:canSave
				bind:selected
			/>
		{/await}
		{#snippet actions()}
			{#if mode == 'edit' && path && effectiveWorkspace}
				<WsSpecificVersions
					kind="resource"
					workspaceId={effectiveWorkspace}
					initialPath={path}
					bind:selected
				/>
			{/if}
			<Button
				variant="accent"
				unifiedSize="md"
				startIcon={{ icon: Save }}
				on:click={() => {
					resourceEditor?.save()
					drawer?.closeDrawer()
				}}
				disabled={!canSave}
			>
				Save
			</Button>
		{/snippet}
	</DrawerContent>
</Drawer>
