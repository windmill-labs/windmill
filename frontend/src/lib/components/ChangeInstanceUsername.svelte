<script lang="ts">
	import { SettingService, UserService } from '$lib/gen'
	import { Button, Popup } from './common'
	import { sendUserToast } from '$lib/toast'
	import Alert from './common/alert/Alert.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { createEventDispatcher } from 'svelte'

	export let email: string
	export let username: string
	export let isConflict = false

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
		const automateUsernameCreation =
			(await SettingService.getGlobal({ key: 'automate_username_creation' })) ?? false

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

		sendUserToast(`Rename user ${email} to ${username}`)

		dispatch('renamed')
	}
</script>

<Popup
	floatingConfig={{
		middleware: [
			autoPlacement({
				allowedPlacements: ['bottom-end', 'top-end']
			})
		]
	}}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button color={isConflict ? 'red' : 'light'} size="xs" spacingSize="xs2" nonCaptureEvent={true}
			>{isConflict ? 'Fix username conflict' : 'Change username'}</Button
		>
	</svelte:fragment>
	<div class="flex flex-col max-w-2xl p-2">
		<span class="text-sm mb-2 leading-6 font-semibold"
			>{isConflict ? 'Fix username conflict' : 'Change username'}</span
		>

		<span class="text-xs mb-1 leading-6"
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
			the workspace, and references in resources to variables. You will have to handle those
			manually.
			<br />
		</Alert>

		<Button
			variant="contained"
			color="blue"
			size="xs"
			on:click={() => {
				renameUser().then(() => {
					close(null)
				})
			}}
			disabled={email === undefined || !username}
		>
			Confirm username change
		</Button>
	</div>
</Popup>
