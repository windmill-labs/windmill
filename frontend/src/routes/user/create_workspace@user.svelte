<script lang="ts">
	import { goto } from '$app/navigation'

	import { UserService, WorkspaceService } from '../../gen'
	import { logoutWithRedirect, sendUserToast } from '../../utils'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '../../stores'
	import CenteredModal from './CenteredModal.svelte'

	let id = ''
	let name = ''
	let username = ''
	let domain = ''

	let errorId = ''

	$: id = name.toLowerCase().replace(/\s/gi, '-')

	$: validateName(id)

	async function validateName(id: string): Promise<void> {
		try {
			await WorkspaceService.validateId({ requestBody: { id } })
			errorId = ''
		} catch {
			errorId = 'id already exists'
		}
	}
	async function createWorkspace(): Promise<void> {
		try {
			await WorkspaceService.createWorkspace({
				requestBody: {
					id,
					name,
					username,
					domain
				}
			})
			sendUserToast(`Successfully created workspace id: ${id}`)
			usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			workspaceStore.set(id)
			goto('/')
		} catch (err) {
			console.error(err)
			sendUserToast(`Cannot create workspace: ${err.body}`, true)
		}
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key || event.keyCode
		if (key === 13 || key === 'Enter') {
			event.preventDefault()
			createWorkspace()
		}
	}

	async function loadWorkspaces() {
		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			} catch {}
		}
		if (!$usersWorkspaceStore) {
			logoutWithRedirect($page.url.pathname)
		}
	}

	loadWorkspaces()

	UserService.globalWhoami().then((x) => {
		if (x.name) {
			username = x.name.split(' ')[0]
		} else {
			username = x.email.split('@')[0]
		}
		username = username.toLowerCase()
	})
</script>

<CenteredModal title="Create a new workspace">
	<label class="block pb-2">
		<span class="text-gray-700">workspace name:</span>
		<input bind:value={name} class="default-input" />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700">workspace id:</span>
		{#if errorId}
			<span class="text-red-500 text-xs">{errorId}</span>
		{/if}
		<input bind:value={id} class="default-input" class:input-error={errorId != ''} />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700">your username in that workspace:</span>
		<input bind:value={username} on:keyup={handleKeyUp} class="default-input" />
	</label>
	<div class="flex flex-row justify-between pt-4">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
		<button
			disabled={errorId != '' || !name || !username || !id}
			class="default-button"
			type="button"
			on:click={createWorkspace}
		>
			Create workspace
		</button>
	</div>
</CenteredModal>
