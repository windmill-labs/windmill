<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'

	import { SettingService, UserService, WorkspaceService } from '$lib/gen'
	import { validateUsername } from '$lib/utils'
	import { page } from '$app/stores'
	import { usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { Button } from '$lib/components/common'

	let workspace_id = $page.url.searchParams.get('workspace') ?? ''
	let username = ''
	let errorUsername = ''
	let checking = false

	$: validateName(username)

	async function acceptInvite(): Promise<void> {
		await UserService.acceptInvite({
			requestBody: {
				username: automateUsernameCreation ? undefined : username,
				workspace_id
			}
		})
		usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
		workspaceStore.set(workspace_id)
		goto($page.url.searchParams.get('rd') ?? '/')
	}

	async function validateName(username: string): Promise<void> {
		checking = true
		try {
			await WorkspaceService.existsUsername({ requestBody: { id: workspace_id, username } })
			errorUsername = validateUsername(username)
		} catch (error) {
			errorUsername = 'Username already exists'
		}
		checking = false
	}

	function handleKey(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			acceptInvite()
		}
	}

	let automateUsernameCreation = true
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? true

		if (!automateUsernameCreation) {
			UserService.globalWhoami().then((x) => {
				let uname = ''
				if (x.name) {
					uname = x.name.split(' ')[0]
				} else {
					uname = x.email.split('@')[0]
				}
				username = uname.toLowerCase()
			})
		}
	}
	getAutomateUsernameCreationSetting()
</script>

<!-- Enable submit form on enter -->

<CenteredModal title="Invitation to join {workspace_id}">
	{#if !automateUsernameCreation}
		<label class="block pb-2">
			<span class="text-secondary text-sm">Your username in workspace {workspace_id}:</span>
			<input on:keyup={handleKey} bind:value={username} class:input-error={errorUsername != ''} />
			{#if errorUsername}
				<span class="text-red-500 text-xs">{errorUsername}</span>
			{/if}
		</label>
	{/if}
	<div class="flex flex-row justify-between pt-4 gap-x-1">
		<Button variant="default" unifiedSize="md" href="{base}/user/workspaces"
			>&leftarrow; Back to workspaces</Button
		>
		<Button
			disabled={checking || (!automateUsernameCreation && (errorUsername != '' || !username))}
			variant="accent"
			unifiedSize="md"
			onClick={acceptInvite}
		>
			Accept invite
		</Button>
	</div>
</CenteredModal>
