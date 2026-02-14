<script lang="ts">
	import { workspaceStore, superadmin } from '$lib/stores'
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

			sendUserToast(`Moved workspace to ${newName}. Old workspace archived. Reloading...`)
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

<div class="flex flex-col gap-1">
	<p class="font-semibold text-xs text-emphasis">Workspace ID</p>
	<p class="text-xs text-secondary font-normal">Slug to uniquely identify your workspace</p>
	<div class="flex flex-row gap-0.5 items-center">
		<p class="text-xs font-normal text-primary">{$workspaceStore ?? ''}</p>
		{#if !isCloudHosted() || $superadmin}
			<Button
				on:click={() => {
					open = true
				}}
				unifiedSize="sm"
				variant="subtle"
				iconOnly
				startIcon={{
					icon: Pen
				}}
			/>
		{/if}
	</div>
</div>

<Modal bind:open title="Change workspace ID">
	<div class="flex flex-col gap-4">
		<Alert type="warning" title="What happens">
			<ul class="list-disc list-inside text-xs mt-1 space-y-1">
				<li>All content (scripts, flows, apps, resources, etc.) moves to the new ID</li>
				<li>Old workspace is archived with completed jobs, logs, and audit history</li>
				<li>Running jobs will be canceled</li>
			</ul>
			<p class="text-xs mt-2">Remember to update webhook URLs and CLI sync config afterward.</p>
		</Alert>
		<p class="text-secondary text-xs"
			>Current ID <br /> <span class="text-emphasis">{$workspaceStore ?? ''}</span></p
		>
		<label class="flex flex-col gap-1">
			<span class="text-emphasis text-xs">New name</span>
			<input type="text" bind:value={newName} />
		</label>
		<label class="block">
			<span class="text-emphasis text-xs">New ID</span>
			<input type="text" bind:value={newId} />
			{#if errorId}
				<div class="text-red-500 text-xs mt-1">{errorId}</div>
			{/if}
		</label>
	</div>

	{#snippet actions()}
		<Button
			variant="accent"
			disabled={checking || errorId.length > 0 || !newName || !newId}
			{loading}
			on:click={() => {
				renameWorkspace()
			}}
		>
			Save
		</Button>
	{/snippet}
</Modal>
