<script lang="ts">
	import { goto } from '$app/navigation'

	import { UserService, WorkspaceService } from '$lib/gen'
	import { validateUsername } from '$lib/utils'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'

	let workspace_id = $page.url.searchParams.get('workspace') ?? ''
	let username = ''
	let errorUsername = ''

	$: validateName(username)

	async function acceptInvite(): Promise<void> {
		await UserService.acceptInvite({
			requestBody: {
				username,
				workspace_id
			}
		})
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		workspaceStore.set(workspace_id)
		goto($page.url.searchParams.get('rd') ?? '/scripts')
	}

	async function validateName(username: string): Promise<void> {
		try {
			await WorkspaceService.existsUsername({ requestBody: { id: workspace_id, username } })
			errorUsername = validateUsername(username)
		} catch (error) {
			errorUsername = 'username already exists'
		}
	}

	function handleKey(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			acceptInvite()
		}
	}

	UserService.globalWhoami().then((x) => {
		if (x.name) {
			username = x.name.split(' ')[0]
		} else {
			username = x.email.split('@')[0]
		}
		username = username.toLowerCase()
	})
</script>

<!-- Enable submit form on enter -->

<CenteredModal title="Invitation to join {workspace_id}">
	<label class="block pb-2">
		<span class="text-gray-700">Your username in workspace {workspace_id}:</span>
		{#if errorUsername}
			<span class="text-red-500 text-xs">{errorUsername}</span>
		{/if}
		<input
			on:keyup={handleKey}
			bind:value={username}
			class="mt-1"
			class:input-error={errorUsername != ''}
		/>
	</label>
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="border" size="sm" href="/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
		<button
			disabled={errorUsername != '' || !username}
			class="place-items-end bg-blue-500 hover:bg-blue-700 text-white font-bold py-1 px-2 border rounded"
			type="button"
			on:click={acceptInvite}
		>
			Accept invite
		</button>
	</div>
</CenteredModal>
