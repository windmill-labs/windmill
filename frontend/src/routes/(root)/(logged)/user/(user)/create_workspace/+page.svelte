<script lang="ts">
	import { goto } from '$app/navigation'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { sendUserToast, validateUsername } from '$lib/utils'
	import { logoutWithRedirect } from '$lib/logout'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { onMount } from 'svelte'

	const rd = $page.url.searchParams.get('rd')

	let id = ''
	let name = ''
	let username = ''

	let errorId = ''
	let errorUser = ''
	let checking = false

	$: id = name.toLowerCase().replace(/\s/gi, '-')

	$: validateName(id)
	$: errorUser = validateUsername(username)

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

	async function createWorkspace(): Promise<void> {
		await WorkspaceService.createWorkspace({
			requestBody: {
				id,
				name,
				username
			}
		})
		if (auto_invite) {
			await WorkspaceService.editAutoInvite({
				workspace: id,
				requestBody: { operator: operatorOnly }
			})
		}
		sendUserToast(`Created workspace id: ${id}`)

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
			await logoutWithRedirect($page.url.pathname + $page.url.search)
		}
	}

	onMount(() => {
		loadWorkspaces()

		UserService.globalWhoami().then((x) => {
			if (x.name) {
				username = x.name.split(' ')[0]
			} else {
				username = x.email.split('@')[0]
			}
			username = username.toLowerCase()
		})

		WorkspaceService.isDomainAllowed().then((x) => {
			isDomainAllowed = x
		})
	})

	let isDomainAllowed = false

	$: domain = $usersWorkspaceStore?.email.split('@')[1]

	let auto_invite = false
	let operatorOnly = false
</script>

<CenteredModal title="New Workspace">
	<label class="block pb-2 pt-4">
		<span class="text-gray-700 text-sm">Workspace name</span>
		<input type="text" bind:value={name} />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700 text-sm">Workspace ID</span>
		{#if errorId}
			<span class="text-red-500 text-xs">{errorId}</span>
		{/if}
		<input type="text" bind:value={id} class:input-error={errorId != ''} />
	</label>
	<label class="block pb-2">
		<span class="text-gray-700 text-sm">Your username in that workspace</span>
		{#if errorUser}
			<span class="text-red-500 text-xs">{errorUser}</span>
		{/if}
		<input type="text" bind:value={username} on:keyup={handleKeyUp} />
	</label>
	<Toggle
		disabled={!isDomainAllowed}
		bind:checked={auto_invite}
		options={{ right: `Auto invite users with the same email address domain (${domain})` }}
	/>
	<div class="flex items-center gap-1">
		<Toggle
			disabled={!auto_invite}
			bind:checked={operatorOnly}
			options={{ right: `Auto invite users as operators` }}
		/>
		<Tooltip
			>An operator can only execute and view scripts/flows/apps from your workspace, and only
			those that he has visibility on</Tooltip
		>
	</div>
	{#if !isDomainAllowed}
		<div class="text-gray-600 text-sm mb-4 mt-2">{domain} domain not allowed for auto-invite</div>
	{/if}
	<div class="flex flex-wrap flex-row justify-between pt-10 gap-1">
		<Button variant="border" size="sm" href="/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
		<Button
			disabled={checking || errorId != '' || errorUser != '' || !name || !username || !id}
			on:click={createWorkspace}
		>
			Create workspace
		</Button>
	</div>
</CenteredModal>
