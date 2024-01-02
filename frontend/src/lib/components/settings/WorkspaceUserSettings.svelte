<script lang="ts">
	import AddUser from '$lib/components/AddUser.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'

	import InviteUser from '$lib/components/InviteUser.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'

	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import type { User } from '$lib/gen'
	import { UserService, WorkspaceService, type WorkspaceInvite } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Search } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import Cell from '../table/Cell.svelte'
	import Row from '../table/Row.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { isCloudHosted } from '$lib/cloud'

	let users: User[] | undefined = undefined
	let invites: WorkspaceInvite[] = []
	let filteredUsers: User[] | undefined = undefined
	let userFilter = ''
	let auto_invite_domain: string | undefined
	let operatorOnly: boolean | undefined = undefined
	let nbDisplayed = 30

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		auto_invite_domain = settings.auto_invite_domain
		operatorOnly = settings.auto_invite_operator
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsers({ workspace: $workspaceStore! })
	}

	async function listInvites(): Promise<void> {
		invites = await WorkspaceService.listPendingInvites({ workspace: $workspaceStore! })
	}

	let allowedAutoDomain = false

	async function getDisallowedAutoDomain() {
		allowedAutoDomain = await WorkspaceService.isDomainAllowed()
	}

	$: domain = $userStore?.email.split('@')[1]

	$: {
		if ($workspaceStore) {
			getDisallowedAutoDomain()
			listUsers()
			listInvites()
			loadSettings()
		}
	}

	let deleteConfirmedCallback: (() => void) | undefined = undefined

	async function removeAllInvitesFromDomain() {
		await Promise.all(
			invites
				.filter((x) => (isCloudHosted() ? x.email.endsWith('@' + auto_invite_domain ?? '') : true))
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

	let nbInviteDisplayed = 50
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<div class="flex flex-row justify-between items-center">
	<PageHeader
		title="Members ({filteredUsers?.length ?? users?.length ?? ''})"
		primary={false}
		tooltip="Manage users manually or enable SSO authentication."
		documentationLink="https://www.windmill.dev/docs/core_concepts/authentification"
	/>

	<div class="flex flex-row items-center gap-2 relative">
		<input placeholder="Filter members" bind:value={userFilter} class="input !pl-8" />
		<Search class="absolute left-2" size={14} />

		<AddUser on:new={listUsers} />
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
				{#each filteredUsers.slice(0, nbDisplayed) as { email, username, is_admin, operator, usage, disabled } (email)}
					<tr class="!hover:bg-surface-hover">
						<Cell first>{email}</Cell>
						<Cell>{username}</Cell>
						<Cell>{usage?.executions}</Cell>
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
									<ToggleButton
										value="operator"
										size="xs"
										label="Operator"
										tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
									/>

									<ToggleButton
										value="developer"
										size="xs"
										label="Developer"
										tooltip="A Developer can execute and view scripts/flows/apps, but they can also create new ones and edit those they are allowed to by their path (either u/ or Writer or Admin of their folder found at /f)."
									/>

									<ToggleButton
										value="admin"
										size="xs"
										label="Admin"
										tooltip="An admin has full control over a specific Windmill workspace, including the ability to manage users, edit entities, and control permissions within the workspace."
									/>
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

<PageHeader
	title="Invites ({invites.length ?? ''})"
	primary={false}
	tooltip="Manage invites on your workspace."
	documentationLink="https://www.windmill.dev/docs/core_concepts/authentification#adding-users-to-a-workspace"
>
	<div class="flex gap-8 items-center">
		{#if auto_invite_domain != undefined}
			<Toggle
				size="sm"
				bind:checked={operatorOnly}
				options={{
					right: `Auto-invited users to join as operators`
				}}
				on:change={async (e) => {
					console.log(e.detail)
					await removeAllInvitesFromDomain()
					await WorkspaceService.editAutoInvite({
						workspace: $workspaceStore ?? '',
						requestBody: { operator: e.detail, invite_all: !isCloudHosted() }
					})
					loadSettings()
					listInvites()
				}}
			/>
		{/if}
		<Toggle
			size="sm"
			checked={auto_invite_domain != undefined}
			on:change={async (e) => {
				await removeAllInvitesFromDomain()
				await WorkspaceService.editAutoInvite({
					workspace: $workspaceStore ?? '',
					requestBody: e.detail
						? { operator: operatorOnly ?? false, invite_all: !isCloudHosted() }
						: { operator: undefined }
				})
				loadSettings()
				listInvites()
			}}
			disabled={!allowedAutoDomain}
			options={{
				right: isCloudHosted()
					? `Auto-invite anyone from ${
							auto_invite_domain != undefined ? auto_invite_domain : domain
					  }`
					: 'Auto-invite anyone joining the instance'
			}}
		/>
		{#if !allowedAutoDomain}
			<div class="text-red-400 text-xs mb-2">{domain} domain not allowed for auto-invite</div>
		{/if}
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
									<ToggleButton
										value="operator"
										size="xs"
										label="Operator"
										tooltip="An operator can only execute and view scripts/flows/apps from your workspace, and only those that he has visibility on."
									/>

									<ToggleButton
										value="developer"
										size="xs"
										label="Developer"
										tooltip="A Developer can execute and view scripts/flows/apps, but they can also create new ones and edit those they are allowed to by their path (either u/ or Writer or Admin of their folder found at /f)."
									/>

									<ToggleButton
										value="admin"
										size="xs"
										label="Admin"
										tooltip="An admin has full control over a specific Windmill workspace, including the ability to manage users, edit entities, and control permissions within the workspace."
									/>
								</ToggleButtonGroup>
							</div>
						</Cell>
						<Cell last>
							<button
								class="ml-2 text-red-500"
								on:click={async () => {
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
			<button class="ml-4" on:click={() => (nbInviteDisplayed += 50)}>load 50 more</button></span
		>
	{/if}
</div>

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
