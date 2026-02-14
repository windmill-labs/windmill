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
	<p class="font-semibold text-xs text-emphasis">Workspace name</p>
	<p class="text-xs text-secondary font-normal">Displayable name</p>
	<div class="flex flex-row gap-2 items-center">
		<p class="text-primary text-xs">{currentName}</p>
		<Button
			on:click={() => {
				open = true
			}}
			unifiedSize="sm"
			iconOnly
			variant="subtle"
			startIcon={{
				icon: Pen
			}}
		/>
	</div>
</div>

<Modal bind:open title="Change workspace name">
	<div class="flex flex-col gap-4 mt-4">
		{#if currentName}
			<p class="text-secondary text-xs"
				>Current name <br /> <span class="text-emphasis">{currentName}</span></p
			>
		{/if}
		<label class="flex flex-col gap-1">
			<span class="text-emphasis text-xs">New name</span>
			<input type="text" bind:value={newName} />
		</label>
	</div>

	{#snippet actions()}
		<Button
			size="sm"
			variant="accent"
			disabled={!newName}
			on:click={() => {
				renameWorkspace()
			}}
		>
			Save
		</Button>
	{/snippet}
</Modal>
