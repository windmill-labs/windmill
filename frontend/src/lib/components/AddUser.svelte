<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { globalEmailInvite, superadmin, workspaceStore } from '$lib/stores'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { Button, Popup } from './common'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$app/navigation'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { UserPlus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	let email: string
	let username: string

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addUser()
		}
	}

	async function addUser() {
		await WorkspaceService.addUser({
			workspace: $workspaceStore!,
			requestBody: {
				email,
				username,
				is_admin: selected == 'admin',
				operator: selected == 'operator'
			}
		})
		sendUserToast(`Added ${email}`)
		if (!(await UserService.existsEmail({ email }))) {
			let isSuperadmin = $superadmin
			if (!isCloudHosted()) {
				sendUserToast(
					`User ${email} is not registered yet on the instance. ${
						!isSuperadmin
							? `If not using SSO, ask an administrator to add ${email} to the instance`
							: ''
					}`,
					true,
					isSuperadmin
						? [
								{
									label: 'Add user to the instance',
									callback: () => {
										$globalEmailInvite = email
										goto('#superadmin-settings')
									}
								}
						  ]
						: []
				)
			}
		}
		dispatch('new')
	}

	let selected: 'operator' | 'author' | 'admin' = 'author'
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: UserPlus }}>
			Add new user
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-72 p-2">
		<span class="text-sm mb-2 leading-6 font-semibold">Add a new user</span>

		<span class="text-xs mb-1 leading-6">Email</span>
		<input type="email mb-1" on:keyup={handleKeyUp} placeholder="email" bind:value={email} />

		<span class="text-xs mb-1 pt-2 leading-6">Username</span>
		<input type="text" on:keyup={handleKeyUp} placeholder="username" bind:value={username} />

		<span class="text-xs mb-1 pt-2 leading-6">Role</span>
		<ToggleButtonGroup bind:selected class="mb-4">
			<ToggleButton
				value="operator"
				size="sm"
				label="Operator"
				tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
			/>
			<ToggleButton
				position="center"
				value="author"
				size="sm"
				label="Author"
				tooltip="An Author can execute and view scripts/flows/apps, but they can also create new ones."
			/>
			<ToggleButton
				position="right"
				value="admin"
				size="sm"
				label="Admin"
				tooltip="An admin has full control over a specific Windmill workspace, including the ability to manage users, edit entities, and control permissions within the workspace."
			/>
		</ToggleButtonGroup>
		<Button
			variant="contained"
			color="blue"
			size="sm"
			on:click={() => {
				addUser().then(() => {
					// @ts-ignore
					email = undefined
					// @ts-ignore
					username = undefined
				})
			}}
			disabled={email === undefined || username === undefined}
		>
			Add
		</Button>
	</div>
</Popup>
