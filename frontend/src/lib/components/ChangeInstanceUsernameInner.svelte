<script lang="ts">
	import { SettingService, UserService } from '$lib/gen'
	import { Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import Alert from './common/alert/Alert.svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		email: string
		username: string
		isConflict?: boolean
		noPadding?: boolean
	}

	let { email, username = $bindable(), isConflict = false, noPadding = false }: Props = $props()

	let loading = $state(false)

	type UsernameInfo = {
		username: string
		workspace_usernames: {
			workspace_id: string
			username: string
		}[]
	}

	let usernameInfo: UsernameInfo | undefined = $state(undefined)

	let affectedWorkspaces = $derived.by(
		() => usernameInfo?.workspace_usernames.filter((w) => w.username !== username) ?? []
	)
	let isRenaming = $derived.by(
		() =>
			usernameInfo !== undefined &&
			(username !== usernameInfo.username || affectedWorkspaces.length > 0)
	)

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
		// Renaming before the current usernames are known would apply a change whose scope
		// the "Manual action required" warning could not have been shown for.
		if (!usernameInfo) {
			return
		}
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
		onkeyup={handleKeyUp}
		bind:value={username}
		disabled={isConflict}
	/>

	{#if isConflict}
		<Alert title="Username conflict" class="mb-4">
			Users are required to have an instance-wide username that is shared across all workspaces.
			However, this user has different usernames in different workspaces.

			{#if affectedWorkspaces.length > 0}
				<br />
				<br />
				Workspaces requiring username modification: {affectedWorkspaces
					.map((wu) => `${wu.workspace_id} (${wu.username})`)
					.join(', ')}
			{/if}
		</Alert>
	{/if}

	{#if !isConflict && affectedWorkspaces.length > 0}
		<Alert title="Concerned workspaces" class="mb-4">
			{affectedWorkspaces.map((wu) => `${wu.workspace_id}`).join(', ')}
		</Alert>
	{/if}

	{#if isRenaming}
		<Alert type="warning" title="Manual action required" class="mb-4">
			This operation does not handle references in scripts, workflows and applications to scripts in
			the workspace, and references in resources to variables. You will have to handle those
			manually.
			<br />
		</Alert>
	{/if}

	<Button
		variant="default"
		unifiedSize="md"
		on:click={() => {
			renameUser().then(() => {
				dispatch('close')
			})
		}}
		disabled={email === undefined || !username || !usernameInfo}
		{loading}
	>
		Confirm username change
	</Button>
</div>
