<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { globalEmailInvite, superadmin, workspaceStore } from '$lib/stores'
	import { SettingService, UserService, WorkspaceService } from '$lib/gen'
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$lib/navigation'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { UserPlus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()

	let email: string | undefined = $state()
	let username: string | undefined = $state()

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addUser()
		}
	}

	let automateUsernameCreation = $state(true)
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? true
	}
	getAutomateUsernameCreationSetting()

	async function addUser() {
		await WorkspaceService.addUser({
			workspace: $workspaceStore!,
			requestBody: {
				email: email!,
				username: automateUsernameCreation ? undefined : username,
				is_admin: selected == 'admin',
				operator: selected == 'operator'
			}
		})
		sendUserToast(`Added ${email}`)
		if (!(await UserService.existsEmail({ email: email! }))) {
			let isSuperadmin = $superadmin
			if (!isCloudHosted()) {
				const emailCopy = email!
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
										$globalEmailInvite = emailCopy
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

	let selected: 'operator' | 'developer' | 'admin' = $state('developer')
</script>

<Popover placement="bottom-end">
	{#snippet trigger()}
		<Button variant="accent" unifiedSize="md" nonCaptureEvent={true} startIcon={{ icon: UserPlus }}>
			Add new user
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col w-72 p-4">
			<span class="text-sm mb-2 leading-6 font-semibold">Add a new user</span>

			<span class="text-xs mb-1 leading-6">Email</span>
			<input type="email mb-1" onkeyup={handleKeyUp} placeholder="email" bind:value={email} />

			{#if !automateUsernameCreation}
				<span class="text-xs mb-1 pt-2 leading-6">Username</span>
				<input type="text" onkeyup={handleKeyUp} placeholder="username" bind:value={username} />
			{/if}

			<span class="text-xs mb-1 pt-6 leading-6">Role</span>
			<ToggleButtonGroup bind:selected class="mb-4">
				{#snippet children({ item })}
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
				{/snippet}
			</ToggleButtonGroup>
			<Button
				variant="accent"
				size="sm"
				on:click={() => {
					addUser().then(() => {
						// @ts-ignore
						email = undefined
						// @ts-ignore
						username = undefined
					})
				}}
				disabled={email === undefined || (!automateUsernameCreation && username === undefined)}
			>
				Add
			</Button>
		</div>
	{/snippet}
</Popover>
