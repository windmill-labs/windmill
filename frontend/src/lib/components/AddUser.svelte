<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { globalEmailInvite, superadmin, workspaceStore, enterpriseLicense } from '$lib/stores'
	import { SettingService, UserService, WorkspaceService } from '$lib/gen'
	import { Button } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { goto } from '$lib/navigation'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
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
		if (selected === 'service_account') {
			if (!username) return
			await WorkspaceService.createServiceAccount({
				workspace: $workspaceStore!,
				requestBody: {
					username: username!,
					is_admin: serviceAccountRole === 'admin',
					operator: serviceAccountRole === 'operator',
					add_to_deployers: serviceAccountRole === 'developer' && addToDeployers
				}
			})
			sendUserToast(`Service account '${username}' created`)
		} else {
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
		}
		dispatch('new')
	}

	type UserRole = 'operator' | 'developer' | 'admin' | 'service_account'
	type ServiceAccountRole = 'operator' | 'developer' | 'admin'
	let selected: UserRole = $state('developer' as UserRole)
	let serviceAccountRole: ServiceAccountRole = $state('operator' as ServiceAccountRole)
	let addToDeployers: boolean = $state(true)
	let isServiceAccount = $derived(selected === 'service_account')
</script>

<Popover placement="bottom-end">
	{#snippet trigger()}
		<Button variant="accent" unifiedSize="md" nonCaptureEvent={true} startIcon={{ icon: UserPlus }}>
			Add new user
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex flex-col w-[28rem] p-4">
			<span class="text-sm mb-2 leading-6 font-semibold">Add a new user</span>

			{#if isServiceAccount}
				<span class="text-xs mb-1 leading-6">Username</span>
				<input
					type="text"
					onkeyup={handleKeyUp}
					placeholder="my_service_account"
					autocomplete="off"
					data-1p-ignore
					bind:value={username}
				/>
			{:else}
				<span class="text-xs mb-1 leading-6">Email</span>
				<input type="email mb-1" onkeyup={handleKeyUp} placeholder="email" bind:value={email} />

				{#if !automateUsernameCreation}
					<span class="text-xs mb-1 pt-2 leading-6">Username</span>
					<input type="text" onkeyup={handleKeyUp} placeholder="username" bind:value={username} />
				{/if}
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
					<ToggleButton
						value="service_account"
						label={$enterpriseLicense ? 'Service Account' : 'Service Account (EE)'}
						tooltip="A service account is a workspace-scoped identity for automation. It cannot log in directly and can be impersonated by admins."
						disabled={!$enterpriseLicense}
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>

			{#if isServiceAccount}
				<span class="text-xs mb-1 leading-6">Service account role</span>
				<ToggleButtonGroup bind:selected={serviceAccountRole} class="mb-4">
					{#snippet children({ item })}
						<ToggleButton
							value="operator"
							label="Operator"
							tooltip="Read/run only. Counts as 0.5 seat. Cannot be used for CLI sync or to author scripts/flows/apps."
							{item}
						/>
						<ToggleButton
							value="developer"
							label="Developer"
							tooltip="Can author and edit scripts/flows/apps within its path. Counts as 1 seat. Use this for CLI sync tokens."
							{item}
						/>
						<ToggleButton
							value="admin"
							label="Admin"
							tooltip="Full workspace admin. Counts as 1 seat. Grant only when the service account needs to manage workspace settings."
							{item}
						/>
					{/snippet}
				</ToggleButtonGroup>

				{#if serviceAccountRole === 'developer'}
					<div class="flex items-center gap-2 mb-4">
						<Toggle bind:checked={addToDeployers} size="xs" />
						<span class="text-xs leading-6">
							Add to <code>wm_deployers</code>
							<Tooltip>
								Recommended when this service account will be used as a <code>wmill sync push</code>
								/ CI deploy identity. Members of <code>wm_deployers</code> can deploy on behalf of
								other users in the target workspace.
								<a
									href="https://www.windmill.dev/docs/core_concepts/staging_prod#run-on-behalf-of"
									target="_blank"
									rel="noopener noreferrer"
									class="underline">Learn more</a
								>.
							</Tooltip>
						</span>
					</div>
				{/if}
			{/if}
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
				disabled={isServiceAccount
					? username === undefined || username === ''
					: email === undefined || (!automateUsernameCreation && username === undefined)}
			>
				Add
			</Button>
		</div>
	{/snippet}
</Popover>
