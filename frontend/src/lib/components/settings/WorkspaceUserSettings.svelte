<script lang="ts">
	import AddUser from '$lib/components/AddUser.svelte'
	import { Alert, Badge, Button, Section, Skeleton } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import WorkspaceOperatorSettings from '$lib/components/settings/WorkspaceOperatorSettings.svelte'
	import InviteUser from '$lib/components/InviteUser.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'

	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { CancelablePromise, User, UserUsage } from '$lib/gen'
	import { UserService, WorkspaceService, GroupService, type WorkspaceInvite } from '$lib/gen'
	import { userStore, workspaceStore, superadmin, globalEmailInvite } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Mails, Search, Plus, UserMinus, X } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import SearchItems from '../SearchItems.svelte'
	import Cell from '../table/Cell.svelte'
	import Row from '../table/Row.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { truncate } from '$lib/utils'
	import { onDestroy, untrack } from 'svelte'
	import { goto } from '$lib/navigation'

	let users: User[] | undefined = $state(undefined)
	let invites: WorkspaceInvite[] = $state([])
	let filteredUsers: User[] | undefined = $state(undefined)
	let userFilter = $state('')
	let auto_invite_domain: string | undefined = $state()
	let operatorOnly: boolean | undefined = $state(undefined)
	let autoAdd: boolean | undefined = $state(false)
	let nbDisplayed = $state(30)

	// Instance group auto-add settings
	let instanceGroups: Array<{ name: string; summary?: string; emails?: string[] }> = $state([])
	let autoAddInstanceGroups: string[] = $state([])
	let autoAddInstanceGroupsRoles: Record<string, string> = $state({})

	// Add new instance group form state
	let selectedNewInstanceGroup: string | undefined = $state(undefined)
	let selectedNewRole: string | undefined = $state('developer')

	// Available groups for dropdowns - filter out already configured groups
	let availableGroupItems = $derived(
		instanceGroups
			.filter((group) => !autoAddInstanceGroups.includes(group.name))
			.map((group) => ({
				value: group.name,
				label: group.name + (group.summary ? ` - ${group.summary}` : '')
			}))
	)

	// Sort users so manual users come first, then instance group users
	let sortedUsers = $derived(() => {
		const userList = (filteredUsers || users || []).slice()
		return userList.sort((a: User, b: User) => {
			const aIsInstanceGroup = a.added_via?.source === 'instance_group' ? 1 : 0
			const bIsInstanceGroup = b.added_via?.source === 'instance_group' ? 1 : 0
			return aIsInstanceGroup - bIsInstanceGroup
		})
	})

	let hasNonManualUsers = $derived(
		(filteredUsers || users || []).some(
			(user: User) =>
				user.added_via?.source === 'instance_group' || user.added_via?.source === 'domain'
		)
	)

	// Function to check if a manual user can be converted to a group user
	function canConvertToGroup(user: User): boolean {
		// User must be manually added (not via instance group or domain)
		if (user.added_via?.source === 'instance_group' || user.added_via?.source === 'domain') {
			return false
		}

		// Check if user's email is in any configured instance group
		const userEmail = user.email
		for (const groupName of autoAddInstanceGroups) {
			const group = instanceGroups.find((g) => g.name === groupName)
			if (group && group.emails && group.emails.includes(userEmail)) {
				return true
			}
		}

		return false
	}

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		const autoInvite = settings.auto_invite as
			| {
					enabled?: boolean
					domain?: string
					operator?: boolean
					mode?: string
					instance_groups?: string[]
					instance_groups_roles?: Record<string, string>
			  }
			| undefined
		auto_invite_domain = autoInvite?.enabled ? (autoInvite?.domain ?? '*') : undefined
		operatorOnly = autoInvite?.operator ?? false
		autoAdd = autoInvite?.mode === 'add'
		autoAddInstanceGroups = autoInvite?.instance_groups || []
		autoAddInstanceGroupsRoles = autoInvite?.instance_groups_roles || {}
	}

	let getUsagePromise: CancelablePromise<UserUsage[]> | undefined = undefined

	let usage: Record<string, number> | undefined = $state(undefined)

	async function getUsage() {
		try {
			getUsagePromise = UserService.listUsersUsage({ workspace: $workspaceStore! })
			const res = await getUsagePromise
			usage = res.reduce(
				(acc, { email, executions }) => {
					if (email) {
						acc[email] = executions ?? 0
					}
					return acc
				},
				{} as Record<string, number>
			)
		} catch (e) {
			console.warn(e)
		}
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsers({ workspace: $workspaceStore! })
	}

	async function listInvites(): Promise<void> {
		invites = await WorkspaceService.listPendingInvites({ workspace: $workspaceStore! })
	}

	let allowedAutoDomain = $state(false)

	async function getDisallowedAutoDomain() {
		allowedAutoDomain = await WorkspaceService.isDomainAllowed()
	}

	async function loadInstanceGroups(): Promise<void> {
		try {
			instanceGroups = await GroupService.listInstanceGroups()
		} catch (e) {
			console.warn('Failed to load instance groups:', e)
			instanceGroups = []
		}
	}

	async function saveInstanceGroupSettings(): Promise<void> {
		try {
			await WorkspaceService.editInstanceGroups({
				workspace: $workspaceStore ?? '',
				requestBody: {
					groups: autoAddInstanceGroups,
					roles: autoAddInstanceGroupsRoles
				}
			})
			sendUserToast('Instance group settings saved')
			// Refresh user list to show newly auto-added users
			listUsers()
		} catch (e) {
			console.error('Failed to save instance group settings:', e)
			sendUserToast('Failed to save settings', true)
		}
	}

	async function addInstanceGroup(): Promise<void> {
		if (!selectedNewInstanceGroup || !selectedNewRole) return

		const groupToAdd = selectedNewInstanceGroup
		const roleToAdd = selectedNewRole

		try {
			autoAddInstanceGroups = [...autoAddInstanceGroups, groupToAdd]
			autoAddInstanceGroupsRoles[groupToAdd] = roleToAdd

			// Reset form
			selectedNewInstanceGroup = undefined
			selectedNewRole = 'developer'

			await saveInstanceGroupSettings()
		} catch (e) {
			// Rollback on error
			autoAddInstanceGroups = autoAddInstanceGroups.filter((g) => g !== groupToAdd)
			delete autoAddInstanceGroupsRoles[groupToAdd]
			sendUserToast('Failed to add instance group', true)
		}
	}

	async function removeInstanceGroup(groupName: string): Promise<void> {
		const previousGroups = [...autoAddInstanceGroups]
		const previousRole = autoAddInstanceGroupsRoles[groupName]

		try {
			autoAddInstanceGroups = autoAddInstanceGroups.filter((g) => g !== groupName)
			delete autoAddInstanceGroupsRoles[groupName]
			await saveInstanceGroupSettings()
		} catch (e) {
			// Rollback on error
			autoAddInstanceGroups = previousGroups
			if (previousRole) {
				autoAddInstanceGroupsRoles[groupName] = previousRole
			}
			sendUserToast('Failed to remove instance group', true)
		}
	}

	async function updateGroupRole(groupName: string, role: string): Promise<void> {
		const previousRole = autoAddInstanceGroupsRoles[groupName]

		try {
			autoAddInstanceGroupsRoles[groupName] = role
			await saveInstanceGroupSettings()
		} catch (e) {
			// Rollback on error
			autoAddInstanceGroupsRoles[groupName] = previousRole
			sendUserToast('Failed to update role', true)
		}
	}

	async function convertUserToGroup(username: string): Promise<void> {
		try {
			await UserService.convertUserToGroup({
				workspace: $workspaceStore ?? '',
				username
			})
			sendUserToast('User converted to group user')
			listUsers()
		} catch (e) {
			console.error('Failed to convert user:', e)
			sendUserToast('Failed to convert user', true)
		}
	}

	let domain = $derived($userStore?.email.split('@')[1])

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				getDisallowedAutoDomain()
				listUsers()
				getUsage()
				listInvites()
				loadSettings()
				loadInstanceGroups()
			})
		}
	})

	onDestroy(() => {
		try {
			getUsagePromise?.cancel()
		} catch (e) {
			console.warn(e)
		}
	})

	let deleteConfirmedCallback: (() => void) | undefined = $state(undefined)
	let removeInstanceGroupConfirmedCallback: (() => void) | undefined = $state(undefined)
	let convertConfirmedCallback: (() => void) | undefined = $state(undefined)

	// Auto-add/invite confirmation modal states
	let autoAddConfirmCallback: (() => void) | undefined = $state(undefined)
	let autoInviteDisableConfirmCallback: (() => void) | undefined = $state(undefined)
	let switchToAutoAddConfirmCallback: (() => void) | undefined = $state(undefined)

	async function removeAllInvitesFromDomain() {
		await Promise.all(
			invites
				.filter((x) =>
					isCloudHosted() ? x.email.endsWith('@' + (auto_invite_domain ?? '')) : true
				)
				.map(({ email, is_admin, operator }) =>
					WorkspaceService.deleteInvite({
						workspace: $workspaceStore ?? '',
						requestBody: {
							email,
							is_admin,
							operator
						}
					})
				)
		)
	}

	let nbInviteDisplayed = $state(50)

	async function inviteUser(email: string, selected: 'operator' | 'developer' | 'admin') {
		try {
			await WorkspaceService.inviteUser({
				workspace: $workspaceStore!,
				requestBody: {
					email,
					is_admin: selected == 'admin',
					operator: selected == 'operator'
				}
			})
			sendUserToast(`Invited ${email}`)
		} catch (e) {
			console.error('Failed to invite user:', e)
			sendUserToast('Failed to invite user', true)
		}

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

		listInvites()
	}

	async function updateAutoInvite(enable: boolean) {
		// Cleanup invites if auto add is enabled
		if (enable && autoAdd) {
			await removeAllInvitesFromDomain()
		}
		const updateType = enable ? (autoInviteOrAddEnabled ? 'update' : 'enable') : 'disable'
		try {
			// await removeAllInvitesFromDomain()
			await WorkspaceService.editAutoInvite({
				workspace: $workspaceStore ?? '',
				requestBody: enable
					? {
							operator: operatorOnly ?? false,
							invite_all: !isCloudHosted(),
							auto_add: autoAdd
						}
					: {
							operator: undefined,
							auto_add: undefined
						}
			})
			const message =
				updateType === 'update'
					? `Auto-${autoAdd ? 'add' : 'invite'} updated`
					: updateType === 'enable'
						? `Auto-${autoAdd ? 'add' : 'invite'} enabled`
						: `Auto-${autoAdd ? 'add' : 'invite'} disabled`
			sendUserToast(message)
		} catch (e) {
			console.error('Failed to update auto invite:', e)
			sendUserToast('Failed to update auto invite', true)
		}
		loadSettings()
		listInvites()
		listUsers()
	}

	const autoInviteOrAddEnabled = $derived(auto_invite_domain != undefined)

	// Legacy auto-invite: user already has auto-invite enabled (not auto-add) on a non-cloud instance
	// This preserves their existing setup even though auto-invite is deprecated for new setups
	const isLegacyAutoInvite = $derived(autoInviteOrAddEnabled && !autoAdd && !isCloudHosted())

	// Show auto-invite toggle only for:
	// - Cloud hosted users (always available)
	// - Legacy users who already have auto-invite enabled (preserve existing setup)
	const showAutoInviteToggle = $derived(isCloudHosted() || isLegacyAutoInvite)

	// Display mode for labels: for non-cloud, non-legacy users, always show "add"
	// For cloud or legacy users, show based on actual autoAdd setting
	const displayMode = $derived.by(() => {
		if (!isCloudHosted() && !isLegacyAutoInvite) {
			return 'add'
		}
		return autoAdd ? 'add' : 'invite'
	})
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<SettingsPageHeader
	title="Members {(filteredUsers?.length ?? users?.length) != undefined
		? `(${filteredUsers?.length ?? users?.length})`
		: ''}"
	description="Add members to your workspace and manage their roles. You can also auto-add users to join your workspace."
	link="https://www.windmill.dev/docs/core_concepts/roles_and_permissions"
/>

<Section>
	{#snippet action()}
		<div class="flex flex-row items-center gap-2 relative whitespace-nowrap w-full">
			<input placeholder="Filter members" bind:value={userFilter} class="input !pl-8 !w-56" />
			<Search class="absolute left-2" size={14} />

			<Popover
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
				usePointerDownOutside
			>
				{#snippet trigger()}
					<Button
						variant="default"
						unifiedSize="md"
						nonCaptureEvent={true}
						startIcon={{ icon: Mails }}
						>Auto-{displayMode}: {autoInviteOrAddEnabled ? 'ON' : 'OFF'}
					</Button>
				{/snippet}
				{#snippet content()}
					<div class="flex flex-col items-start p-4 min-w-[320px] max-w-sm">
						{#if showAutoInviteToggle}
							<div class="text-xs mb-1 text-primary"
								>Mode <Tooltip>Whether to invite or add users directly to the workspace.</Tooltip>
							</div>
							<ToggleButtonGroup
								selected={displayMode}
								on:selected={async (e) => {
									const switchingToAdd = e.detail === 'add' && !autoAdd

									// If switching from invite to add on non-cloud, show confirmation with warning
									if (switchingToAdd && isLegacyAutoInvite) {
										switchToAutoAddConfirmCallback = async () => {
											autoAdd = true
											if (autoInviteOrAddEnabled) {
												await updateAutoInvite(true)
											}
										}
									} else {
										autoAdd = e.detail === 'add'
										if (autoInviteOrAddEnabled) {
											await updateAutoInvite(true)
										}
									}
								}}
							>
								{#snippet children({ item })}
									<ToggleButton value="invite" small label="Auto-invite" {item} />
									<ToggleButton value="add" small label="Auto-add" {item} />
								{/snippet}
							</ToggleButtonGroup>

							{#if isLegacyAutoInvite && !autoAdd}
								<div class="mt-3 w-full">
									<Alert type="warning" size="xs" title="Legacy mode">
										Auto-invite is deprecated. Switching to auto-add will permanently disable
										auto-invite for this workspace.
									</Alert>
								</div>
							{/if}

							<div class="mt-6"></div>
						{/if}

						<span class="text-xs mb-1">Role <Tooltip>Role of the auto-added users</Tooltip></span>
						<ToggleButtonGroup
							selected={operatorOnly ? 'operator' : 'developer'}
							on:selected={async (e) => {
								operatorOnly = e.detail === 'operator'
								if (auto_invite_domain != undefined) {
									await updateAutoInvite(true)
								}
							}}
						>
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
							{/snippet}
						</ToggleButtonGroup>

						<div class="mt-6">
							<Toggle
								checked={autoInviteOrAddEnabled}
								on:change={async (e) => {
									const enabling = e.detail

									if (enabling) {
										// Non-cloud users without legacy auto-invite: force auto-add mode
										if (!isCloudHosted() && !isLegacyAutoInvite) {
											autoAdd = true
										}

										// Show confirmation when enabling auto-add
										if (autoAdd || (!isCloudHosted() && !showAutoInviteToggle)) {
											autoAddConfirmCallback = async () => {
												await updateAutoInvite(true)
											}
										} else {
											await updateAutoInvite(true)
										}
									} else {
										// Disabling: show confirmation if currently using auto-invite (legacy)
										if (isLegacyAutoInvite) {
											autoInviteDisableConfirmCallback = async () => {
												await updateAutoInvite(false)
											}
										} else {
											await updateAutoInvite(false)
										}
									}
								}}
								disabled={isCloudHosted() && !allowedAutoDomain}
								options={{
									right: isCloudHosted()
										? `Auto-${displayMode} anyone from ${
												autoInviteOrAddEnabled ? auto_invite_domain : domain
											}`
										: `Auto-${displayMode} anyone joining the instance`
								}}
							/>
						</div>
						{#if isCloudHosted() && !allowedAutoDomain}
							<div class="text-red-400 text-xs">{domain} domain not allowed for auto-add</div>
						{/if}
					</div>
				{/snippet}
			</Popover>

			{#if instanceGroups.length > 0}
				<Popover
					floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
					usePointerDownOutside
					floatingClass="!z-20"
				>
					{#snippet trigger()}
						<Button
							color={autoAddInstanceGroups.length > 0 ? 'green' : 'gray'}
							variant="border"
							size="xs"
							nonCaptureEvent={true}
							startIcon={{ icon: Mails }}
							>Instance groups: {autoAddInstanceGroups.length}
						</Button>
					{/snippet}
					{#snippet content()}
						<div class="flex flex-col p-4 min-w-[500px]">
							<div class="flex flex-col gap-4">
								<span class="text-sm leading-6 font-semibold"> Auto-add instance groups </span>

								<!-- Add new instance group form -->
								{#if availableGroupItems.length > 0}
									<div class="flex w-full mt-1 gap-2 items-end justify-between">
										<div class="flex gap-2 items-end">
											<div class="flex flex-col gap-1">
												<span class="text-xs text-primary">Instance group</span>
												<Select
													items={availableGroupItems}
													placeholder="Select group"
													bind:value={selectedNewInstanceGroup}
													class="max-w-[160px]"
													disablePortal={true}
												/>
											</div>

											<div class="flex flex-col gap-1">
												<span class="text-xs text-primary">Role</span>
												<ToggleButtonGroup
													selected={selectedNewRole}
													on:selected={(e) => {
														selectedNewRole = e.detail
													}}
												>
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
											</div>
										</div>

										<Button
											color="blue"
											size="xs"
											startIcon={{ icon: Plus }}
											disabled={!selectedNewInstanceGroup || !selectedNewRole}
											onclick={addInstanceGroup}
										>
											Add
										</Button>
									</div>
								{/if}

								<!-- Configured groups table -->
								{#if autoAddInstanceGroups.length > 0}
									<div class="flex flex-col gap-2">
										<p class="text-sm font-medium text-secondary">Configured groups:</p>
										<div class="flex flex-col gap-1">
											<table class="w-full text-sm">
												<thead>
													<tr class="text-left text-xs text-primary">
														<th class="pb-2 w-1/2">Group</th>
														<th class="pb-2 w-1/4">Role</th>
														<th class="pb-2 w-1/4"></th>
													</tr>
												</thead>
												<tbody>
													{#each autoAddInstanceGroups as groupName (groupName)}
														{@const group = instanceGroups.find((g) => g.name === groupName)}
														<tr class="border-t border-gray-200 dark:border-gray-700">
															<td class="py-2">
																<div class="font-medium">{groupName}</div>
																{#if group?.summary}
																	<div class="text-xs text-primary">{group.summary}</div>
																{/if}
															</td>
															<td class="py-2">
																<div>
																	<ToggleButtonGroup
																		selected={autoAddInstanceGroupsRoles[groupName] || 'developer'}
																		on:selected={async (e) => {
																			autoAddInstanceGroupsRoles[groupName] = e.detail
																			await updateGroupRole(groupName, e.detail)
																		}}
																	>
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
																</div>
															</td>
															<td class="py-2">
																<div class="flex justify-end">
																	<Button
																		color="light"
																		variant="contained"
																		btnClasses="text-red-500"
																		size="xs"
																		spacingSize="xs2"
																		onclick={() => {
																			removeInstanceGroupConfirmedCallback = async () => {
																				await removeInstanceGroup(groupName)
																			}
																		}}
																	>
																		Remove
																	</Button>
																</div>
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									</div>
								{:else}
									<div class="text-center text-primary text-sm py-4">
										No instance groups configured for auto-add
									</div>
								{/if}
							</div>
						</div>
					{/snippet}
				</Popover>
			{/if}

			{#if showAutoInviteToggle}
				<InviteUser {inviteUser} />
			{/if}

			<AddUser
				on:new={() => {
					listUsers()
					listInvites()
				}}
			/>
		</div>
	{/snippet}

	<DataTable
		shouldLoadMore={(filteredUsers?.length ?? 0) > 30}
		loadMore={30}
		on:loadMore={() => {
			nbDisplayed += 30
		}}
	>
		<Head>
			<tr>
				<Cell head first>Email</Cell>
				<Cell head>Username</Cell>
				{#if hasNonManualUsers}
					<Cell head>
						Added via
						<Tooltip>
							Shows how the user was added to the workspace: manually, via domain auto-invite, or
							through an instance group.
						</Tooltip>
					</Cell>
				{/if}

				<Cell head>
					Executions (<abbr title="past 1w">1w</abbr>)
					<Tooltip>
						An execution is calculated as 1 for any runs of scripts + 1 for each seconds above the
						first one
					</Tooltip>
				</Cell>
				<Cell head>Role</Cell>
				<Cell head>Enabled</Cell>
				<Cell head last>
					<span class="sr-only">Actions</span>
				</Cell>
			</tr>
		</Head>
		<tbody>
			{#if filteredUsers}
				{#each sortedUsers().slice(0, nbDisplayed) as user, index (user.email)}
					{@const { email, username, is_admin, operator, disabled, added_via } = user}
					<!-- Add separator between manual users and instance group users -->
					{#if hasNonManualUsers && index > 0 && sortedUsers()[index - 1]?.added_via?.source !== 'instance_group' && added_via?.source === 'instance_group'}
						<tr class="bg-surface-secondary">
							<td colspan={hasNonManualUsers ? 8 : 7} class="px-4 py-2">
								<div class="text-xs text-emphasis font-semibold"> Instance group users </div>
							</td>
						</tr>
					{/if}
					<tr class={index % 2 === 0 ? 'bg-surface-tertiary' : 'bg-surface'}>
						<Cell first><a href="mailto:{email}">{truncate(email, 20)}</a></Cell>
						<Cell>{truncate(username, 30)}</Cell>
						{#if hasNonManualUsers}
							<Cell>
								<div class="flex items-center gap-2">
									{#if added_via?.source === 'instance_group'}
										<Badge color="blue">Group</Badge>
										<span>{truncate(added_via.group || 'Unknown', 20)}</span>
									{:else if added_via?.source === 'domain'}
										<Badge color="blue">Auto-add</Badge>
									{:else}
										<Badge color="blue">Manual</Badge>
									{/if}
								</div>
							</Cell>
						{/if}
						<Cell
							>{#if usage?.[email] != undefined}{usage?.[email]}{:else}<Loader2
									size={14}
									class="animate-spin"
								/>{/if}</Cell
						>
						<Cell>
							<div>
								{#if added_via?.source === 'instance_group'}
									<div class="flex items-center gap-1">
										<span class="rounded-md text-xs px-2 py-1 bg-surface shadow-md font-bold">
											{is_admin ? 'Admin' : operator ? 'Operator' : 'Developer'}
										</span>
										<Tooltip>Role is managed through instance group configuration above.</Tooltip>
									</div>
								{:else}
									<ToggleButtonGroup
										selected={is_admin ? 'admin' : operator ? 'operator' : 'developer'}
										on:selected={async (e) => {
											if (is_admin && email == $userStore?.email && e.detail != 'admin') {
												sendUserToast(
													'Admins cannot be demoted by themselves, ask another admin to demote you',
													true
												)
												e.preventDefault()
												listUsers()
												return
											}
											const body =
												e.detail == 'admin'
													? { is_admin: true, operator: false }
													: e.detail == 'operator'
														? { is_admin: false, operator: true }
														: { is_admin: false, operator: false }
											await UserService.updateUser({
												workspace: $workspaceStore ?? '',
												username,
												requestBody: body
											})
											listUsers()
										}}
									>
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
								{/if}
							</div>
						</Cell>
						<Cell>
							<Toggle
								checked={!disabled}
								on:change={async (e) => {
									try {
										await UserService.updateUser({
											workspace: $workspaceStore ?? '',
											username,
											requestBody: {
												disabled: !disabled
											}
										})
										sendUserToast(`User ${username} ${disabled ? 'enabled' : 'disabled'}`)
										listUsers()
									} catch (e) {
										console.error('Failed to update user status:', e)
										sendUserToast('Failed to update user status', true)
									}
								}}
								size="xs"
							/>
						</Cell>
						<Cell>
							<div class="flex gap-1">
								{#snippet removeUserButton(disabled: boolean)}
									<Button
										unifiedSize="sm"
										variant="subtle"
										destructive
										{disabled}
										onClick={() => {
											deleteConfirmedCallback = async () => {
												await UserService.deleteUser({
													workspace: $workspaceStore ?? '',
													username
												})
												sendUserToast('User removed')
												listUsers()
											}
										}}
										startIcon={{ icon: UserMinus }}
									>
										Remove
									</Button>
								{/snippet}

								{#if added_via?.source === 'instance_group'}
									<div class="flex items-center gap-1">
										{@render removeUserButton(true)}
										<Tooltip
											>Cannot remove users synced from instance groups. Either disable the user or
											remove them from the SCIM group.</Tooltip
										>
									</div>
								{:else if canConvertToGroup(user)}
									<Button
										variant="accent"
										unifiedSize="sm"
										on:click={() => {
											convertConfirmedCallback = async () => {
												await convertUserToGroup(username)
											}
										}}
									>
										Convert
									</Button>
								{:else}
									{@render removeUserButton(false)}
								{/if}
							</div>
						</Cell>
					</tr>
				{/each}
			{:else}
				{#each new Array(6) as _}
					<tr class="border">
						<td colspan={6}>
							<Skeleton layout={[[4]]} />
						</td>
					</tr>
				{/each}
			{/if}
		</tbody>
	</DataTable>
</Section>

<div class="pt-12"></div>

<WorkspaceOperatorSettings />

<div class="pt-12"></div>

{#if invites?.length > 0}
	<Section
		label="Invites ({invites.length ?? ''})"
		tooltip="Manage invites on your workspace."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification#adding-users-to-a-workspace"
	>
		{#snippet action()}
			{#if showAutoInviteToggle}
				<div class="flex gap-2 items-center">
					<InviteUser {inviteUser} />
				</div>
			{/if}
		{/snippet}
		<DataTable>
			<Head>
				<tr>
					<Cell head first>Email</Cell>
					<Cell head>Role</Cell>
					<Cell head last><span class="sr-only">Actions</span></Cell>
				</tr>
			</Head>
			<tbody class="divide-y bg-surface">
				{#if invites?.length > 0}
					{#each invites.slice(0, nbInviteDisplayed) as { email, is_admin, operator }}
						<Row>
							<Cell first>{email}</Cell>
							<Cell>
								<div>
									<ToggleButtonGroup
										selected={is_admin ? 'admin' : operator ? 'operator' : 'developer'}
										on:selected={async (e) => {
											const body =
												e.detail == 'admin'
													? { is_admin: true, operator: false }
													: e.detail == 'operator'
														? { is_admin: false, operator: true }
														: { is_admin: false, operator: false }
											await WorkspaceService.inviteUser({
												workspace: $workspaceStore ?? '',
												requestBody: {
													email,
													...body
												}
											})
											listUsers()
										}}
									>
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
								</div>
							</Cell>
							<Cell last>
								<Button
									variant="default"
									destructive
									unifiedSize="sm"
									startIcon={{ icon: X }}
									btnClasses="w-fit"
									onClick={async () => {
										await WorkspaceService.deleteInvite({
											workspace: $workspaceStore ?? '',
											requestBody: {
												email,
												is_admin,
												operator
											}
										})
										listInvites()
									}}
								>
									Cancel invite
								</Button>
							</Cell>
						</Row>
					{/each}
				{:else}
					<tr>
						<td colspan="3" class="text-center py-8">
							<div class="text-xs text-secondary"> No invites yet </div>
						</td>
					</tr>
				{/if}
			</tbody>
		</DataTable>
		{#if invites && invites?.length > 50 && nbInviteDisplayed < invites.length}
			<span class="text-xs"
				>{nbInviteDisplayed} invites out of {invites.length}
				<button class="ml-4" onclick={() => (nbInviteDisplayed += 50)}>load 50 more</button></span
			>
		{/if}
	</Section>
{/if}
<ConfirmationModal
	open={Boolean(deleteConfirmedCallback)}
	title="Remove user"
	confirmationText="Remove"
	on:canceled={() => {
		deleteConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (deleteConfirmedCallback) {
			deleteConfirmedCallback()
		}
		deleteConfirmedCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove ?</span>
	</div>
</ConfirmationModal>

<div class="[&>div]:!z-[5002]">
	<ConfirmationModal
		open={Boolean(removeInstanceGroupConfirmedCallback)}
		title="Remove instance group"
		confirmationText="Remove"
		on:canceled={() => {
			removeInstanceGroupConfirmedCallback = undefined
		}}
		on:confirmed={() => {
			if (removeInstanceGroupConfirmedCallback) {
				removeInstanceGroupConfirmedCallback()
			}
			removeInstanceGroupConfirmedCallback = undefined
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span
				>Are you sure you want to remove this instance group from auto-add? This will not remove
				users already added from this group.</span
			>
		</div>
	</ConfirmationModal>
</div>

<ConfirmationModal
	open={Boolean(convertConfirmedCallback)}
	title="Convert to Group User"
	confirmationText="Convert"
	on:canceled={() => {
		convertConfirmedCallback = undefined
	}}
	on:confirmed={() => {
		if (convertConfirmedCallback) {
			convertConfirmedCallback()
		}
		convertConfirmedCallback = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to convert this user to a group user?</span>
		<span class="text-sm text-secondary">This will:</span>
		<ul class="text-sm text-secondary list-disc ml-4 space-y-1">
			<li>Change the user's role based on their instance group configuration</li>
			<li>Make their role managed through the instance group settings</li>
			<li>Prevent manual role changes for this user</li>
		</ul>
	</div>
</ConfirmationModal>

<!-- Auto-add/invite confirmation modals - z-index to appear above popover -->
<div class="[&>div]:!z-[5002]">
	<ConfirmationModal
		open={Boolean(autoAddConfirmCallback)}
		title="Enable Auto-add"
		confirmationText="Enable"
		on:canceled={() => {
			autoAddConfirmCallback = undefined
		}}
		on:confirmed={() => {
			if (autoAddConfirmCallback) {
				autoAddConfirmCallback()
			}
			autoAddConfirmCallback = undefined
		}}
	>
		Are you sure you want to enable auto-add?<br />
		Anyone added to the instance will automatically join this workspace.
	</ConfirmationModal>
</div>

<div class="[&>div]:!z-[5002]">
	<ConfirmationModal
		open={Boolean(autoInviteDisableConfirmCallback)}
		title="Disable Auto-invite"
		confirmationText="Disable"
		on:canceled={() => {
			autoInviteDisableConfirmCallback = undefined
		}}
		on:confirmed={() => {
			if (autoInviteDisableConfirmCallback) {
				autoInviteDisableConfirmCallback()
			}
			autoInviteDisableConfirmCallback = undefined
		}}
	>
		Are you sure you want to disable auto-invite? Auto-invite is a legacy feature. After disabling,
		it will no longer be available for this workspace. You will only be able to use auto-add. <br />
		Anyone added to the instance will automatically join this workspace.
	</ConfirmationModal>
</div>

<div class="[&>div]:!z-[5002]">
	<ConfirmationModal
		open={Boolean(switchToAutoAddConfirmCallback)}
		title="Switch to Auto-add"
		confirmationText="Switch"
		on:canceled={() => {
			switchToAutoAddConfirmCallback = undefined
		}}
		on:confirmed={() => {
			if (switchToAutoAddConfirmCallback) {
				switchToAutoAddConfirmCallback()
			}
			switchToAutoAddConfirmCallback = undefined
		}}
	>
		Are you sure you want to switch from auto-invite to auto-add?<br />
		Auto-invite is a legacy feature. After switching to auto-add, auto-invite will no longer be available
		for this workspace. <br />
		With auto-add, anyone added to the instance will automatically join this workspace without needing
		to accept an invitation.
	</ConfirmationModal>
</div>
