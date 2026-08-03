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
	import Select from './select/Select.svelte'
	import { resource } from 'runed'

	const dispatch = createEventDispatcher()

	let email: string | undefined = $state()
	let username: string | undefined = $state()
	let emailFilterText: string = $state('')
	let typedEmail: string = $state('')
	let popoverOpen: boolean = $state(false)
	let dropdownOpen: boolean = $state(false)
	let addedCount: number = $state(0)

	// Only superadmins may list the instance users, everyone else just types the email in.
	// The list stays capped: what is typed narrows it server-side rather than client-side.
	const instanceUsers = resource(
		() => ({
			search: emailFilterText,
			isSuperadmin: $superadmin,
			open: popoverOpen,
			workspace: $workspaceStore,
			addedCount
		}),
		async ({ search, isSuperadmin, open, workspace }, _previous, { onCleanup }) => {
			if (!isSuperadmin || !open || !workspace) return []
			const request = UserService.listAddableInstanceUsers({
				workspace,
				perPage: 10,
				search: search || undefined
			})
			// A superseded run is only aborted through the promise it registers here. Without this a
			// slow earlier search can land last and overwrite the results of the newer one.
			onCleanup(() => request.cancel())
			return await request
		},
		{ debounce: 300, initialValue: [] }
	)

	let emailItems = $derived(
		instanceUsers.current.map((u) => ({
			value: u.email,
			label: u.username ? `${u.username} (${u.email})` : u.email
		}))
	)

	const EMAIL_REGEX = /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/
	// A whole address typed into the picker is submittable on its own, so `Add` stays reachable
	// without going through the "Add new" row. A search term that is not an address is not: it
	// would be submitted verbatim and rejected instead of adding whoever the list is showing.
	let effectiveEmail = $derived(
		email ??
			(EMAIL_REGEX.test(typedEmail.trim().toLowerCase())
				? typedEmail.trim().toLowerCase()
				: undefined)
	)

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
			// Read once: the picker's own state changes as soon as the request lands.
			const email = effectiveEmail!
			await WorkspaceService.addUser({
				workspace: $workspaceStore!,
				requestBody: {
					email,
					username: automateUsernameCreation ? undefined : username,
					is_admin: selected == 'admin',
					operator: selected == 'operator'
				}
			})
			sendUserToast(`Added ${email}`)
			// The picker excludes members server-side, so it has to refetch to drop this one.
			addedCount++
			if (!(await UserService.existsEmail({ email }))) {
				let isSuperadmin = $superadmin
				if (!isCloudHosted()) {
					const emailCopy = email
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

<Popover placement="bottom-end" bind:isOpen={popoverOpen} onClose={() => (typedEmail = '')}>
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
				<!-- `loading` is deliberately not passed: Select disables its input while loading, which
				would blur the field on every search-as-you-type round trip. -->
				<Select
					bind:value={email}
					bind:open={
						() => dropdownOpen,
						(v) => {
							dropdownOpen = v
							// A closed Select renders its value, not the search text, so an address that
							// stays submittable has to become the value — otherwise `Add` is armed with
							// something no longer on screen.
							if (!v) {
								const typed = typedEmail.trim().toLowerCase()
								if (EMAIL_REGEX.test(typed)) email = typed
								typedEmail = ''
							}
						}
					}
					bind:filterText={
						() => emailFilterText,
						(v) => {
							emailFilterText = v
							// Only the user's own edits are kept: Select clears this text when it closes or
							// when a row is picked, which must not wipe the address about to be submitted.
							if (dropdownOpen) {
								typedEmail = v
								if (v) email = undefined
							}
						}
					}
					items={emailItems}
					placeholder="email"
					clearable
					onCreateItem={(e) => (email = e.trim().toLowerCase())}
					noItemsMsg={instanceUsers.loading
						? 'Loading...'
						: $superadmin
							? 'No user found on the instance'
							: 'Type an email address'}
				/>

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
						email = undefined
						typedEmail = ''
						username = undefined
					})
				}}
				disabled={isServiceAccount
					? username === undefined || username === ''
					: effectiveEmail === undefined || (!automateUsernameCreation && username === undefined)}
			>
				Add
			</Button>
		</div>
	{/snippet}
</Popover>
