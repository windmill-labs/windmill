<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { superadmin, workspaceStore } from '$lib/stores'
	import { UserService, WorkspaceService } from '$lib/gen'
	import { Button, ToggleButton, ToggleButtonGroup } from './common'
	import Tooltip from './Tooltip.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$app/navigation'

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

<div class="flex flex-row">
	<input type="email" on:keyup={handleKeyUp} placeholder="email" bind:value={email} class="mr-4" />
	<input
		type="text"
		on:keyup={handleKeyUp}
		placeholder="username"
		bind:value={username}
		class="mr-4"
	/>
	<ToggleButtonGroup bind:selected>
		<ToggleButton position="left" value="operator" size="sm"
			>Operator <Tooltip
				>An operator can only execute and view scripts/flows/apps from your workspace, and only
				those that he has visibility on</Tooltip
			></ToggleButton
		>
		<ToggleButton position="center" value="author" size="sm"
			>Author <Tooltip
				>An Author can execute and view scripts/flows/apps, but he can also create new ones</Tooltip
			></ToggleButton
		>
		<ToggleButton position="right" value="admin" size="sm">Admin</ToggleButton>
	</ToggleButtonGroup>
	<Button
		variant="contained"
		color="blue"
		size="sm"
		btnClasses="!ml-8"
		on:click={addUser}
		disabled={email === undefined}
	>
		Add
	</Button>
</div>
