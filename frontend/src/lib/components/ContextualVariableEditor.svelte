<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import { createEventDispatcher } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { Button } from './common'
	import Drawer from './common/drawer/Drawer.svelte'
	import DrawerContent from './common/drawer/DrawerContent.svelte'
	import autosize from 'svelte-autosize'
	import { sendUserToast } from '$lib/toast'
	import Section from './Section.svelte'
	import { Save } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	let edit: boolean = false
	let name: string = ''
	let value: string = ''

	export function initNew(): void {
		edit = false
		name = ''
		value = ''
		drawer.openDrawer()
	}

	export function editVariable(editName: string, editValue: string): void {
		edit = true
		name = editName
		value = editValue
		drawer.openDrawer()
	}

	let drawer: Drawer

	async function updateVariable(): Promise<void> {
		await WorkspaceService.setEnvironmentVariable({
			workspace: $workspaceStore!,
			requestBody: {
				value: value,
				name: name
			}
		})
		sendUserToast(`${edit ? 'Updated' : 'Created'} contextual variable ${name}`)
		dispatch('update')
		drawer.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="900px">
	<DrawerContent
		title={edit ? `Update contextual variable ${name}` : 'Create a contextual variable'}
		on:close={drawer.closeDrawer}
	>
		<div class="flex flex-col gap-8">
			{#if !edit}
				<Section label="Name">
					<input type="text" bind:value={name} placeholder="Variable name" />
				</Section>
			{/if}
			<Section label="Value">
				<textarea rows="4" type="text" use:autosize bind:value placeholder="Variable value" />
			</Section>
		</div>
		<svelte:fragment slot="actions">
			<Button
				on:click={() => updateVariable()}
				disabled={value === '' || name === ''}
				startIcon={{ icon: Save }}
				color="dark"
				size="sm"
			>
				{edit ? 'Update' : 'Save'}
			</Button>
		</svelte:fragment>
	</DrawerContent>
</Drawer>
