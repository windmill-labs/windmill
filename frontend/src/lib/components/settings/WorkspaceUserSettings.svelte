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

	let users: User[] | undefined = undefined
	let invites: WorkspaceInvite[] = []
	let filteredUsers: User[] | undefined = undefined
	let userFilter = ''
	let scriptPath: string
	let auto_invite_domain: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'
	let operatorOnly: boolean | undefined = undefined
	let nbDisplayed = 30
	let customer_id: string | undefined = undefined
	let webhook: string | undefined = undefined
	let workspaceToDeployTo: string | undefined = undefined
	let errorHandlerInitialPath: string
	let errorHandlerScriptPath: string
	let openaiResourceInitialPath: string | undefined = undefined

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		auto_invite_domain = settings.auto_invite_domain
		operatorOnly = settings.auto_invite_operator
		if (settings.slack_command_script) {
			itemKind = settings.slack_command_script.split('/')[0] as 'flow' | 'script'
		}
		scriptPath = (settings.slack_command_script ?? '').split('/').slice(1).join('/')
		customer_id = settings.customer_id
		workspaceToDeployTo = settings.deploy_to
		webhook = settings.webhook
		openaiResourceInitialPath = settings.openai_resource_path
		errorHandlerScriptPath = (settings.error_handler ?? '').split('/').slice(1).join('/')
		errorHandlerInitialPath = errorHandlerScriptPath
	}

	let userLoading: boolean = false

	async function listUsers(): Promise<void> {
		userLoading = true
		users = await UserService.listUsers({ workspace: $workspaceStore! })
		userLoading = false
	}

	let invitesLoading: boolean = false

	async function listInvites(): Promise<void> {
		invitesLoading = true
		invites = await WorkspaceService.listPendingInvites({ workspace: $workspaceStore! })
		invitesLoading = false
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

	async function removeAllInvitesFromDomain() {
		await Promise.all(
			invites
				.filter((x) => x.email.endsWith('@' + auto_invite_domain ?? ''))
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
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<div class="flex flex-row justify-between items-center">
	<PageHeader
		title="Members ({users?.length ?? ''})"
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

<div class="max-h-screen mb-20">
	<DataTable>
		<Head>
			<tr>
				<Cell head first>Email</Cell>
				<Cell head>Username</Cell>

				<Cell head>
					Executions (<abbr title="past 5 weeks">5w</abbr>)
					<Tooltip light>
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
									selected={is_admin ? 'admin' : operator ? 'operator' : 'author'}
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
										value="author"
										size="xs"
										label="Author"
										tooltip="An Author can execute and view scripts/flows/apps, but he can also create new ones."
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
									variant="border"
									size="xs"
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
									color="red"
									variant="border"
									size="xs"
									on:click={async () => {
										await UserService.deleteUser({
											workspace: $workspaceStore ?? '',
											username
										})
										sendUserToast('User removed')
										listUsers()
									}}
								>
									Remove
								</Button>
							</div>
						</Cell>
					</tr>
				{/each}
				{#if filteredUsers?.length > 50}
					<span class="text-xs"
						>{nbDisplayed} items out of {filteredUsers.length}
						<button class="ml-4" on:click={() => (nbDisplayed += 30)}>load 30 more</button></span
					>
				{/if}
			{:else}
				{#each new Array(6) as _}
					<tr class="border">
						{#each new Array(6) as _}
							<td>
								<Skeleton layout={[[2]]} />
							</td>
						{/each}
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
	<InviteUser on:new={listInvites} />
</PageHeader>

<div class="max-h-screen">
	<DataTable>
		<Head>
			<tr>
				<Cell head first>Email</Cell>
				<Cell>head Role</Cell>
				<Cell head last><span class="sr-only">Actions</span></Cell>
			</tr>
		</Head>
		<tbody class="divide-y bg-surface">
			{#if invites?.length > 0}
				{#each invites as { email, is_admin, operator }}
					<tr class="border">
						<Cell first>{email}</Cell>
						<Cell>
							{#if operator}
								<Badge>operator</Badge>
							{:else if is_admin}
								<Badge>admin</Badge>
							{/if}
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
					</tr>
				{/each}
			{:else}
				<tr>
					<td colspan="3" class="text-center py-8">
						<div class="text-xs text-secondary">
							No invites yet. Invite users to your workspace by clicking on the
							<code>Invite</code> button above.
						</div>
					</td>
				</tr>
			{/if}
		</tbody>
	</DataTable>
</div>

<div class="mt-10" />
<PageHeader
	title="Auto Invite"
	tooltip="Auto invite to the workspace users from your domain."
	documentationLink="https://www.windmill.dev/docs/core_concepts/authentification#auto-invite"
	primary={false}
/>
<div class="flex gap-2">
	{#if auto_invite_domain != domain}
		<div>
			<Button
				disabled={!allowedAutoDomain}
				on:click={async () => {
					await WorkspaceService.editAutoInvite({
						workspace: $workspaceStore ?? '',
						requestBody: { operator: false }
					})
					loadSettings()
					listInvites()
				}}>Set auto-invite to {domain}</Button
			>
		</div>
	{/if}
	{#if auto_invite_domain}
		<div class="flex flex-col gap-y-2">
			<Toggle
				bind:checked={operatorOnly}
				options={{
					right: `Auto-invited users to join as operators`
				}}
				on:change={async (e) => {
					await removeAllInvitesFromDomain()
					await WorkspaceService.editAutoInvite({
						workspace: $workspaceStore ?? '',
						requestBody: { operator: e.detail }
					})
					loadSettings()
					listInvites()
				}}
			/>
			<div>
				<Button
					on:click={async () => {
						await removeAllInvitesFromDomain()
						await WorkspaceService.editAutoInvite({
							workspace: $workspaceStore ?? '',
							requestBody: { operator: undefined }
						})
						loadSettings()
						listInvites()
					}}>Unset auto-invite from {auto_invite_domain} domain</Button
				>
			</div>
		</div>
	{/if}
</div>
{#if !allowedAutoDomain}
	<div class="text-red-400 text-xs mb-2">{domain} domain not allowed for auto-invite</div>
{/if}
