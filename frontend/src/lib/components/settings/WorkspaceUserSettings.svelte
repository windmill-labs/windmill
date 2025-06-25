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
	import { UserService, WorkspaceService, type WorkspaceInvite } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Mails, Search } from 'lucide-svelte'
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

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		auto_invite_domain = settings.auto_invite_domain
		operatorOnly = settings.auto_invite_operator
		autoAdd = settings.auto_add
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

	let domain = $derived($userStore?.email.split('@')[1])

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				getDisallowedAutoDomain()
				listUsers()
				getUsage()
				listInvites()
				loadSettings()
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
		<div class="text-tertiary text-xs">
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
		primary={true}
		tooltip="Manage users manually or enable SSO authentication."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification"
	/>

	<div class="flex flex-row items-center gap-2 relative">
		<input placeholder="Filter members" bind:value={userFilter} class="input !pl-8" />
		<Search class="absolute left-2" size={14} />

		<Popover
			floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
			usePointerDownOutside
		>
			{#snippet trigger()}
				<Button
					color={auto_invite_domain != undefined ? 'green' : 'red'}
					variant="border"
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
								<ToggleButton value="invite" size="xs" label="Auto-invite" {item} />
								<ToggleButton value="add" size="xs" label="Auto-add" {item} />
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
										: { operator: undefined, auto_add: undefined }
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
				{#each filteredUsers.slice(0, nbDisplayed) as { email, username, is_admin, operator, disabled } (email)}
					<tr class="!hover:bg-surface-hover">
						<Cell first><a href="mailto:{email}">{truncate(email, 20)}</a></Cell>
						<Cell>{truncate(username, 30)}</Cell>
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
