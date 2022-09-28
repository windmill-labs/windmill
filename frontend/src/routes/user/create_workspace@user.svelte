<script lang="ts">
	import { goto } from '$app/navigation'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { sendUserToast, validateUsername } from '$lib/utils'
	import { logoutWithRedirect } from '$lib/logout'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'

	const rd = $page.url.searchParams.get('rd')

	let id = ''
	let name = ''
	let username = ''
	let domain = ''

	let errorId = ''
	let errorUser = ''

	$: id = name.toLowerCase().replace(/\s/gi, '-')

	$: validateName(id)
	$: errorUser = validateUsername(username)

	async function validateName(id: string): Promise<void> {
		let exists = await WorkspaceService.existsWorkspace({ requestBody: { id } })
		if (exists) {
			errorId = 'id already exists'
		} else if (id != '' && !/^\w+(-\w+)*$/.test(id)) {
			errorId = 'id can only contain letters, numbers and dashes and must not finish by a dash'
		} else {
			errorId = ''
		}
	}

	async function createWorkspace(): Promise<void> {
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
		goto(rd ?? '/')
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
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
			logoutWithRedirect($page.url.pathname + $page.url.search)
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
	{#if $page.url.hostname != 'app.windmill.dev'}
		<div class="bg-blue-100 border-l-4 border-blue-600 text-blue-700 p-4 m-4" role="alert">
			<p class="font-bold">
				More than 1 user-created workspace for self-hosted will require a team or enterprise license
				- Unlimited during beta
			</p>
		</div>
	{/if}

	<label class="block pb-2">
		<span class="text-gray-700">workspace name:</span>
		<input bind:value={name} class="mt-1" />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700">workspace id:</span>
		{#if errorId}
			<span class="text-red-500 text-xs">{errorId}</span>
		{/if}
		<input bind:value={id} class="mt-1" class:input-error={errorId != ''} />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700">your username in that workspace:</span>
		{#if errorUser}
			<span class="text-red-500 text-xs">{errorUser}</span>
		{/if}
		<input bind:value={username} on:keyup={handleKeyUp} class="mt-1" />
	</label>
	<div class="flex flex-row justify-between pt-4">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
		<Button
			disabled={errorId != '' || errorUser != '' || !name || !username || !id}
			on:click={createWorkspace}
		>
			Create workspace
		</Button>
	</div>
</CenteredModal>
