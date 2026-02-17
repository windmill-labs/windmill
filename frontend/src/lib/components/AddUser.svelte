<script lang="ts">
	import { createEventDispatcher, untrack } from 'svelte'
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
	import AutocompleteSelect from './select/AutocompleteSelect.svelte'
	import InputError from './InputError.svelte'
	import TextInput from './text_input/TextInput.svelte'

	const dispatch = createEventDispatcher()

	let { workspaceEmails = [] }: { workspaceEmails?: string[] } = $props()

	let email: string | undefined = $state()
	let username: string | undefined = $state()

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addUser()
		}
	}

	let automateUsernameCreation = $state(false)
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? false
	}
	getAutomateUsernameCreationSetting()

	let allInstanceEmails: string[] | undefined = $state(undefined)
	let emailsLoading = $state(!isCloudHosted())

	let instanceEmails = $derived.by(() => {
		if (!allInstanceEmails) return []
		const workspaceSet = new Set(workspaceEmails)
		return allInstanceEmails
			.filter((e) => !workspaceSet.has(e))
			.map((e) => ({ label: e, value: e }))
	})

	async function loadInstanceEmails() {
		if (isCloudHosted() || !$workspaceStore) return
		try {
			allInstanceEmails = await UserService.listInstanceEmails({ workspace: $workspaceStore })
		} catch {
			allInstanceEmails = undefined
		} finally {
			emailsLoading = false
		}
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadInstanceEmails()
			})
		}
	})

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

	let emailSelect: AutocompleteSelect | undefined = $state()
	let emailError = $derived.by(() => {
		if (!email) return undefined
		const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
		return emailRegex.test(email) ? undefined : 'Please enter a valid email address'
	})
	const displayEmailError = $derived(
		!emailSelect?.getDropdownVisible() && !!emailError ? emailError : undefined
	)

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
			{#if !isCloudHosted()}
				<AutocompleteSelect
					bind:this={emailSelect}
					items={instanceEmails}
					bind:value={email}
					placeholder={emailsLoading ? 'Loading...' : 'Select or type an email'}
					loading={emailsLoading}
					disablePortal={true}
					error={!!displayEmailError}
				/>
			{:else}
				<TextInput
					inputProps={{
						type: 'email',
						onkeyup: handleKeyUp,
						placeholder: 'email'
					}}
					bind:value={email}
					error={displayEmailError ?? ''}
				/>
			{/if}
			<InputError error={displayEmailError} />

			{#if !automateUsernameCreation}
				<span class="text-xs mb-1 pt-2 leading-6">Username</span>
				<TextInput
					inputProps={{ type: 'text', onkeyup: handleKeyUp, placeholder: 'username' }}
					bind:value={username}
				/>
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
				disabled={email === undefined ||
					!!emailError ||
					(!automateUsernameCreation && username === undefined)}
			>
				Add
			</Button>
		</div>
	{/snippet}
</Popover>
