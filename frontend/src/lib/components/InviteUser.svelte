<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { globalEmailInvite, superadmin, workspaceStore } from '$lib/stores'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { Button, Popup } from './common'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$app/navigation'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { MailPlus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	let email: string

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			inviteUser()
		}
	}

	async function inviteUser() {
		await WorkspaceService.inviteUser({
			workspace: $workspaceStore!,
			requestBody: {
				email,
				is_admin: selected == 'admin',
				operator: selected == 'operator'
			}
		})
		sendUserToast(`Invited ${email}`)
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

		email = ''
	}

	let selected: 'operator' | 'author' | 'admin' = 'author'
</script>

<Popup floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	<svelte:fragment slot="button">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: MailPlus }}>
			Invite
		</Button>
	</svelte:fragment>

	<div class="flex flex-col gap-2">
		<input
			type="email"
			on:keyup={handleKeyUp}
			placeholder="email"
			bind:value={email}
			class="mr-4"
		/>
		<ToggleButtonGroup bind:selected>
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
			on:click={inviteUser}
			disabled={email === undefined}
		>
			Invite
		</Button>
	</div>
</Popup>
