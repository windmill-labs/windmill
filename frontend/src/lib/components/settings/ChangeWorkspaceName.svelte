<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import { untrack } from 'svelte'

	let { open = false }: { open?: boolean } = $props()

	let newName = $state('')
	let currentName = $state('')

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => getWorkspaceName())
		}
	})

	async function getWorkspaceName() {
		currentName = await WorkspaceService.getWorkspaceName({ workspace: $workspaceStore! })
	}

	async function renameWorkspace() {
		open = false
		await WorkspaceService.changeWorkspaceName({
			workspace: $workspaceStore!,
			requestBody: {
				new_name: newName
			}
		})

		sendUserToast(`Changed workspace name to ${newName}`)
		newName = ''
		getWorkspaceName()
	}
</script>

<div class="flex flex-col gap-1">
	<p class="font-medium text-xs text-emphasis">Workspace name</p>
	<div class="flex flex-row gap-0.5 items-center">
		<p class="text-xs font-normal text-primary">{currentName}</p>
		<Button
			on:click={() => {
				open = true
			}}
			size="xs"
			spacingSize="xs2"
			color="light"
			iconOnly
			startIcon={{
				icon: Pen
			}}
		/>
	</div>

	<p class="text-2xs text-secondary font-normal"> Displayable name </p>
</div>

<Modal bind:open title="Change workspace name">
	<div class="flex flex-col gap-4 mt-4">
		{#if currentName}
			<p class="text-secondary text-sm"
				>Current name <br /> <span class="font-bold">{currentName}</span></p
			>
		{/if}
		<label class="block">
			<span class="text-secondary text-sm">New name</span>
			<input type="text" bind:value={newName} />
		</label>
	</div>

	<svelte:fragment slot="actions">
		<Button
			size="sm"
			disabled={!newName}
			on:click={() => {
				renameWorkspace()
			}}
		>
			Save
		</Button>
	</svelte:fragment>
</Modal>
