<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'

	let newName = ''
	let newId = ''
	let checking = false
	let errorId = ''

	$: newId = newName.toLowerCase().replace(/\s/gi, '-')

	$: validateName(newId)

	async function validateName(id: string): Promise<void> {
		checking = true
		let exists = await WorkspaceService.existsWorkspace({ requestBody: { id } })
		if (exists) {
			errorId = 'ID already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'ID can only contain letters, numbers and dashes and must not finish by a dash'
		} else {
			errorId = ''
		}
		checking = false
	}

	let loading = false
	async function renameWorkspace() {
		try {
			loading = true
			await WorkspaceService.changeWorkspaceId({
				workspace: $workspaceStore!,
				requestBody: {
					new_name: newName,
					new_id: newId
				}
			})
			open = false

			sendUserToast(`Renamed workspace to ${newName}. Reloading...`)
			await new Promise((resolve) => setTimeout(resolve, 1000))
			window.location.href = '/workspace_settings?tab=general&workspace=' + newId
		} catch (err) {
			sendUserToast(`Error renaming workspace: ${err}`, true)
		} finally {
			loading = false
		}
	}

	export let open = false
</script>

<div>
	<p class="font-semibold text-sm">Workspace ID</p>
	<div class="flex flex-row gap-0.5 items-center">
		<p class="text-sm text-secondary">{$workspaceStore ?? ''}</p>
		{#if !isCloudHosted()}
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
		{/if}
	</div>
	<p class="italic text-xs">Slug to uniquely identify your workspace</p>
</div>

<Modal bind:open title="Change workspace ID">
	<div class="flex flex-col gap-4">
		<Alert type="warning" title="Warning">
			Renaming the workspace may take a few minutes to complete. Once finished, please update your
			webhook calls and adjust your CLI sync configuration accordingly.
		</Alert>
		<p class="text-secondary text-sm"
			>Current ID <br /> <span class="font-bold">{$workspaceStore ?? ''}</span></p
		>
		<label class="block">
			<span class="text-secondary text-sm">New name</span>
			<input type="text" bind:value={newName} />
		</label>
		<label class="block">
			<span class="text-secondary text-sm">New ID</span>
			<input type="text" bind:value={newId} />
			{#if errorId}
				<div class="text-red-500 text-xs mt-1">{errorId}</div>
			{/if}
		</label>
	</div>

	<svelte:fragment slot="actions">
		<Button
			size="sm"
			disabled={checking || errorId.length > 0 || !newName || !newId}
			{loading}
			on:click={() => {
				renameWorkspace()
			}}
		>
			Save
		</Button>
	</svelte:fragment>
</Modal>
