<script lang="ts">
	import { workspaceStore, superadmin } from '$lib/stores'
	import Alert from '../common/alert/Alert.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { WorkspaceService } from '$lib/gen'
	import Modal from '../common/modal/Modal.svelte'
	import { Pen } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'

	let { open = $bindable(false) } = $props()

	let newName = $state('')
	let newId = $derived(newName.toLowerCase().replace(/\s/gi, '-'))
	let checking = $state(false)
	let errorId = $state('')

	$effect(() => {
		validateName(newId)
	})

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

	let loading = $state(false)
	let showMigrationInfo = $state(false)
	let oldWorkspaceId = $state('')

	async function renameWorkspace() {
		try {
			loading = true
			oldWorkspaceId = $workspaceStore!

			await WorkspaceService.migrateWorkspace({
				requestBody: {
					source_workspace_id: $workspaceStore!,
					target_workspace_name: newName,
					target_workspace_id: newId,
					migration_type: 'metadata',
					disable_workspace: true
				}
			})

			open = false
			showMigrationInfo = true

			sendUserToast(`Workspace metadata migrated to ${newId}`)
		} catch (err) {
			sendUserToast(`Error renaming workspace: ${err}`, true)
		} finally {
			loading = false
		}
	}
</script>

<div>
	<p class="font-semibold text-sm">Workspace ID</p>
	<div class="flex flex-row gap-0.5 items-center">
		<p class="text-sm text-secondary">{$workspaceStore ?? ''}</p>
		{#if !isCloudHosted() || $superadmin}
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

{#if showMigrationInfo}
	<Alert type="info" title="Migration Status" class="mt-4">
		<div class="flex flex-col gap-2">
			<p>Workspace metadata has been successfully migrated to <strong>{newId}</strong>.</p>
			<p class="text-sm text-secondary">
				Job history has not been migrated yet. Click the button below to sync jobs from the old
				workspace.
			</p>
			<Button
				size="sm"
				on:click={() => {
					window.location.href = `/workspace_settings/migration?source=${oldWorkspaceId}&workspace=${newId}`
				}}
			>
				Sync Jobs
			</Button>
		</div>
	</Alert>
{/if}
