<script lang="ts">
	import AddUser from '$lib/components/AddUser.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import WorkspaceOperatorSettings from '$lib/components/settings/WorkspaceOperatorSettings.svelte'
	import InviteUser from '$lib/components/InviteUser.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'

	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { CancelablePromise, User, UserUsage } from '$lib/gen'
	import { UserService, WorkspaceService, GroupService, type WorkspaceInvite } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Mails, Search, Plus } from 'lucide-svelte'
	import Select from '$lib/components/select/Select.svelte'
	import SearchItems from '../SearchItems.svelte'
	import Cell from '../table/Cell.svelte'
	import Row from '../table/Row.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { truncate } from '$lib/utils'
	import { onDestroy, untrack } from 'svelte'

	let users: User[] | undefined = $state(undefined)
	let invites: WorkspaceInvite[] = $state([])
	let filteredUsers: User[] | undefined = $state(undefined)
	let userFilter = $state('')
	let auto_invite_domain: string | undefined = $state()
	let operatorOnly: boolean | undefined = $state(undefined)
	let autoAdd: boolean | undefined = $state(undefined)
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
		auto_invite_domain = settings.auto_invite_domain
		operatorOnly = settings.auto_invite_operator
		autoAdd = settings.auto_add
		autoAddInstanceGroups = settings.auto_add_instance_groups || []
		autoAddInstanceGroupsRoles = settings.auto_add_instance_groups_roles || {}
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

	let showInvites = $state(false)
	$effect(() => {
		showInvites = invites?.length > 0 || (auto_invite_domain != undefined && !autoAdd)
	})
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>
<div class="flex flex-col gap-4 my-8">
	<div class="flex flex-col gap-1">
		<div class="text-primary text-xs">
			Add members to your workspace and manage their roles. You can also auto-add users to join your
			workspace.
			<a
				href="https://www.windmill.dev/docs/core_concepts/roles_and_permissions"
				target="_blank"
				class="text-blue-500">Learn more</a
			>.
		</div>
	</div>
</div>
<div class="flex flex-row justify-between items-center pt-2">
	<PageHeader
		title="Members {(filteredUsers?.length ?? users?.length) != undefined
			? `(${filteredUsers?.length ?? users?.length})`
			: ''}"
		primary={false}
		tooltip="Manage users manually or enable SSO authentication."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification"
	/>

	<div class="flex flex-row items-center gap-2 relative whitespace-nowrap">
		<input placeholder="Filter members" bind:value={userFilter} class="input !pl-8" />
		<Search class="absolute left-2" size={14} />

		<Popover
			floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
			usePointerDownOutside
		>
			{#snippet trigger()}
				<Button
					color={'accent'}
					destructive={auto_invite_domain === undefined}
					variant="default"
					size="xs"
					nonCaptureEvent={true}
					startIcon={{ icon: Mails }}
					>Auto-{showInvites && !autoAdd ? 'invite' : 'add'}: {auto_invite_domain != undefined
						? 'ON'
						: 'OFF'}
				</Button>
			{/snippet}
			{#snippet content()}
				<div class="flex flex-col items-start p-4">
					<span class="text-sm leading-6 font-semibold">
						{isCloudHosted()
							? `Auto-add anyone from ${
									auto_invite_domain != undefined ? auto_invite_domain : domain
								}`
							: `Auto-add anyone joining the instance`}
					</span>

					{#if showInvites}
						<span class="text-xs mb-1 leading-6 pt-2"
							>Mode <Tooltip>Whether to invite or add users directly to the workspace.</Tooltip>
						</span>
						<ToggleButtonGroup
							selected={autoAdd ? 'add' : 'invite'}
							on:selected={async (e) => {
								if (auto_invite_domain != undefined) {
									await removeAllInvitesFromDomain()
									await WorkspaceService.editAutoInvite({
										workspace: $workspaceStore ?? '',
										requestBody: {
											operator: operatorOnly ?? false,
											invite_all: !isCloudHosted(),
											auto_add: e.detail === 'add'
										}
									})
									loadSettings()
									listInvites()
									listUsers()
									autoAdd = e.detail === 'add'
								} else {
									autoAdd = e.detail === 'add'
								}
							}}
						>
							{#snippet children({ item })}
								<ToggleButton value="invite" small label="Auto-invite" {item} />
								<ToggleButton value="add" small label="Auto-add" {item} />
							{/snippet}
						</ToggleButtonGroup>
					{/if}

					<span class="text-xs mb-1 leading-6 pt-2"
						>Role <Tooltip>Role of the auto-added users</Tooltip></span
					>
					<ToggleButtonGroup
						selected={operatorOnly ? 'operator' : 'developer'}
						on:selected={async (e) => {
							if (auto_invite_domain != undefined) {
								await removeAllInvitesFromDomain()
								await WorkspaceService.editAutoInvite({
									workspace: $workspaceStore ?? '',
									requestBody: {
										operator: e.detail === 'operator',
										invite_all: !isCloudHosted(),
										auto_add: showInvites ? (autoAdd ?? false) : true
									}
								})
								operatorOnly = e.detail === 'operator'
								loadSettings()
								listInvites()
								listUsers()
							} else {
								operatorOnly = e.detail === 'operator'
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
					<div class="pt-2">
						<Toggle
							size="xs"
							checked={auto_invite_domain != undefined}
							on:change={async (e) => {
								await removeAllInvitesFromDomain()
								await WorkspaceService.editAutoInvite({
									workspace: $workspaceStore ?? '',
									requestBody: e.detail
										? {
												operator: operatorOnly ?? false,
												invite_all: !isCloudHosted(),
												auto_add: showInvites ? (autoAdd ?? false) : true
											}
										: {
												operator: undefined,
												auto_add: undefined
											}
								})
								loadSettings()
								listInvites()
								listUsers()
							}}
							disabled={isCloudHosted() && !allowedAutoDomain}
							options={{
								right: 'Enabled'
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

		<AddUser
			on:new={() => {
				listUsers()
				listInvites()
			}}
		/>
	</div>
</div>

<div class="">
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
				<Cell head>Status</Cell>
				<Cell head>Role</Cell>
				<Cell head last>
					<span class="sr-only">Actions</span>
				</Cell>
			</tr>
		</Head>
		<tbody class="divide-y bg-surface">
			{#if filteredUsers}
				{#each sortedUsers().slice(0, nbDisplayed) as user, index (user.email)}
					{@const { email, username, is_admin, operator, disabled, added_via } = user}
					<!-- Add separator between manual users and instance group users -->
					{#if hasNonManualUsers && index > 0 && sortedUsers()[index - 1]?.added_via?.source !== 'instance_group' && added_via?.source === 'instance_group'}
						<tr class="bg-surface-secondary">
							<td colspan={hasNonManualUsers ? 8 : 7} class="px-4 py-2">
								<div class="text-xs text-primary font-bold"> Instance group users </div>
							</td>
						</tr>
					{/if}
					<tr class="!hover:bg-surface-hover">
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
							<div class="flex gap-1">
								{#if disabled}
									<Badge color="red">Disabled</Badge>
								{:else}
									<Badge color="green">Enabled</Badge>
								{/if}
							</div>
						</Cell>
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
							<div class="flex gap-1">
								<Button
									color="light"
									variant="contained"
									size="xs"
									spacingSize="xs2"
									on:click={async () => {
										await UserService.updateUser({
											workspace: $workspaceStore ?? '',
											username,
											requestBody: {
												disabled: !disabled
											}
										})
										listUsers()
									}}
								>
									{disabled ? 'Enable' : 'Disable'}
								</Button>

								{#if added_via?.source === 'instance_group'}
									<div class="flex items-center gap-1">
										<Button
											color="light"
											variant="contained"
											btnClasses="text-gray-400"
											size="xs"
											spacingSize="xs2"
											disabled={true}
										>
											Remove
										</Button>
										<Tooltip
											>Cannot remove users synced from instance groups. Either disable the user or
											remove them from the SCIM group.</Tooltip
										>
									</div>
								{:else if canConvertToGroup(user)}
									<Button
										color="light"
										variant="contained"
										btnClasses="text-blue-500"
										size="xs"
										spacingSize="xs2"
										on:click={() => {
											convertConfirmedCallback = async () => {
												await convertUserToGroup(username)
											}
										}}
									>
										Convert
									</Button>
									<Button
										color="light"
										variant="contained"
										btnClasses="text-red-500"
										size="xs"
										spacingSize="xs2"
										on:click={() => {
											deleteConfirmedCallback = async () => {
												await UserService.deleteUser({
													workspace: $workspaceStore ?? '',
													username
												})
												sendUserToast('User removed')
												listUsers()
											}
										}}
									>
										Remove
									</Button>
								{:else}
									<Button
										color="light"
										variant="contained"
										btnClasses="text-red-500"
										size="xs"
										spacingSize="xs2"
										on:click={() => {
											deleteConfirmedCallback = async () => {
												await UserService.deleteUser({
													workspace: $workspaceStore ?? '',
													username
												})
												sendUserToast('User removed')
												listUsers()
											}
										}}
									>
										Remove
									</Button>
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
</div>

<WorkspaceOperatorSettings />

{#if showInvites}
	<PageHeader
		title="Invites ({invites.length ?? ''})"
		primary={false}
		tooltip="Manage invites on your workspace."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification#adding-users-to-a-workspace"
	>
		<div class="flex gap-2 items-center">
			<InviteUser on:new={listInvites} />
		</div>
	</PageHeader>

	<div>
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
								<button
									class="ml-2 text-red-500"
									onclick={async () => {
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
									Cancel
								</button>
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
	</div>
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
