<script lang="ts">
	import { SettingService, UserService } from '$lib/gen'
	import { Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import Alert from './common/alert/Alert.svelte'
	import { createEventDispatcher } from 'svelte'

	export let email: string
	export let username: string
	export let isConflict = false
	export let noPadding = false

	let loading = false

	let usernameInfo:
		| {
				username: string
				workspace_usernames: {
					workspace_id: string
					username: string
				}[]
		  }
		| undefined = undefined

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			renameUser()
		}
	}

	async function getUsernameInfo() {
		usernameInfo = await UserService.globalUsernameInfo({
			email
		})
		if (isConflict) {
			username = usernameInfo.username
		}
	}

	getUsernameInfo()

	const dispatch = createEventDispatcher()

	async function renameUser() {
		loading = true
		try {
			const automateUsernameCreation =
				(await SettingService.getGlobal({ key: 'automate_username_creation' })) ?? true

			if (!automateUsernameCreation) {
				sendUserToast(
					'Modifying the username is only possible when the creation of usernames is automated and defined at instance level..'
				)
				return
			}

			await UserService.globalUserRename({
				email,
				requestBody: {
					new_username: username
				}
			})

			sendUserToast(`Renamed user ${email} to ${username}`)

			dispatch('renamed')
		} finally {
			loading = false
		}
	}
</script>

<div class="flex flex-col max-w-2xl {noPadding ? '' : 'p-4'}">
	{#if isConflict}
		<span class="text-sm mb-2 leading-6 font-semibold text-emphasis">Fix username conflict</span>
	{/if}

	<span class="text-xs font-semibold text-emphasis mb-1 leading-6"
		>{isConflict ? 'Auto-generated instance username' : 'New username'}</span
	>
	<input
		type="text"
		class="mb-4"
		on:keyup={handleKeyUp}
		bind:value={username}
		disabled={isConflict}
	/>

	{#if isConflict}
		<Alert title="Username conflict" class="mb-4">
			Users are required to have an instance-wide username that is shared across all workspaces.
			However, this user has different usernames in different workspaces.

			{#if usernameInfo?.workspace_usernames && usernameInfo.workspace_usernames.filter((w) => w.username !== username).length > 0}
				<br />
				<br />
				Workspaces requiring username modification: {usernameInfo.workspace_usernames
					.filter((w) => w.username !== username)
					.map((wu) => `${wu.workspace_id} (${wu.username})`)
					.join(', ')}
			{/if}
		</Alert>
	{/if}

	{#if !isConflict && usernameInfo?.workspace_usernames && usernameInfo.workspace_usernames.filter((w) => w.username !== username).length > 0}
		<Alert title="Concerned workspaces" class="mb-4">
			{usernameInfo.workspace_usernames
				.filter((w) => w.username !== username)
				.map((wu) => `${wu.workspace_id}`)
				.join(', ')}
		</Alert>
	{/if}

	<Alert type="warning" title="Manual action required" class="mb-4">
		This operation does not handle references in scripts, workflows and applications to scripts in
		the workspace, and references in resources to variables. You will have to handle those manually.
		<br />
	</Alert>

	<Button
		variant="default"
		unifiedSize="md"
		on:click={() => {
			renameUser().then(() => {
				dispatch('close')
			})
		}}
		disabled={email === undefined || !username}
		{loading}
	>
		Confirm username change
	</Button>
</div>
