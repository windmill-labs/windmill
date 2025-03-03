<script lang="ts">
	import { sendUserToast } from '$lib/toast'
	import { createEventDispatcher } from 'svelte'
	import { globalEmailInvite, superadmin, workspaceStore } from '$lib/stores'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$lib/navigation'
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

	let selected: 'operator' | 'developer' | 'admin' = 'developer'
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	<svelte:fragment slot="trigger">
		<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: MailPlus }}>
			Invite
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content">
		<div class="flex flex-col gap-2 p-4">
			<input
				type="email"
				on:keyup={handleKeyUp}
				placeholder="email"
				bind:value={email}
				class="mr-4"
			/>
			<ToggleButtonGroup bind:selected let:item>
				<ToggleButton
					value="operator"
					label="Operator"
					tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
					{item}
				/>

				<ToggleButton
					value="developer"
					label="Developer"
					tooltip="A Developer can execute and view scripts/flows/apps, but they can also create new ones and edit those they are allowed to by their path (either u/ or Writer or Admin of their folder found at /f)."
					{item}
				/>

				<ToggleButton
					value="admin"
					label="Admin"
					tooltip="An admin has full control over a specific Windmill workspace, including the ability to manage users, edit entities, and control permissions within the workspace."
					{item}
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
	</svelte:fragment>
</Popover>
