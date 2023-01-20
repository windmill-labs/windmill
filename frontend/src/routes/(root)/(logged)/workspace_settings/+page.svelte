<script lang="ts">
	import {
		UserService,
		type WorkspaceInvite,
		WorkspaceService,
		OauthService,
		Script
	} from '$lib/gen'
	import type { User } from '$lib/gen'
	import { sendUserToast, isCloudHosted } from '$lib/utils'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { superadmin, userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { faSlack } from '@fortawesome/free-brands-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { goto } from '$app/navigation'
	import InviteUser from '$lib/components/InviteUser.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { Alert, Badge, Button, Skeleton, Tab, Tabs } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { faScroll, faBarsStaggered, faExternalLink } from '@fortawesome/free-solid-svg-icons'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import ToggleButtonGroup from '$lib/components/common/toggleButton/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton/ToggleButton.svelte'
	import AddUser from '$lib/components/AddUser.svelte'
	import { page } from '$app/stores'

	let users: User[] | undefined = undefined
	let invites: WorkspaceInvite[] = []
	let filteredUsers: User[] | undefined = undefined
	let userFilter = ''
	let scriptPath: string
	let initialPath: string
	let team_name: string | undefined
	let auto_invite_domain: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'
	let operatorOnly: boolean | undefined = undefined
	let premium_info: { premium: boolean; usage?: number } | undefined = undefined
	let nbDisplayed = 30
	let plan: string | undefined = undefined
	let customer_id: string | undefined = undefined
	let tab: 'users' | 'slack' | 'premium' | 'export_delete' =
		($page.url.searchParams.get('tab') as 'users' | 'slack' | 'premium' | 'export_delete') ??
		'users'

	// function getDropDownItems(username: string): DropdownItem[] {
	// 	return [
	// 		{
	// 			displayName: 'Manage user',
	// 			href: `/admin/user/manage/${username}`
	// 		},
	// 		{
	// 			displayName: 'Delete',
	// 			action: () => deleteUser(username)
	// 		}
	// 	];
	// }

	// async function deleteUser(username: string): Promise<void> {
	// 	try {
	// 		await UserService.deleteUser({ workspace: $workspaceStore!, username });
	// 		users = await UserService.listUsers({ workspace: $workspaceStore! });
	// 		fuse?.setCollection(users);
	// 		sendUserToast(`User ${username} has been removed`);
	// 	} catch (err) {
	// 		console.error(err);
	// 		sendUserToast(`Cannot delete user: ${err}`, true);
	// 	}
	// }

	async function editSlackCommand(): Promise<void> {
		await WorkspaceService.editSlackCommand({
			workspace: $workspaceStore!,
			requestBody: { slack_command_script: `${itemKind}/${scriptPath}` }
		})
		sendUserToast(`slack command script set to ${scriptPath}`)
	}

	async function loadSettings(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		team_name = settings.slack_name
		auto_invite_domain = settings.auto_invite_domain
		operatorOnly = settings.auto_invite_operator
		scriptPath = (settings.slack_command_script ?? '').split('/').slice(1).join('/')
		plan = settings.plan
		customer_id = settings.customer_id
		initialPath = scriptPath
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

	async function loadPremiumInfo() {
		if (isCloudHosted()) {
			premium_info = await WorkspaceService.getPremiumInfo({ workspace: $workspaceStore! })
		}
	}
	$: domain = $userStore?.email.split('@')[1]

	$: {
		if ($workspaceStore) {
			getDisallowedAutoDomain()
			listUsers()
			listInvites()
			loadSettings()
			loadPremiumInfo()
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

	const plans = {
		Free: [
			'Users use their individual global free-tier quotas when doing executions in this workspace',
			'<b>1 000</b> free global executions per-user per month'
		],
		Team: [
			`<b>$10/mo</b> per user in the workspace.`,
			`Executions are not accounted for in the global user's
					quotas but are accounted for in the workspace's quota.`,
			`Every user in the workspace increases the pooled workspace quota by <b>50k</b> executions.`,
			`1 000 executions above the pooled workspace quota is charged 1$.`
		],
		Enterprise: [
			`<b>$50/month</b> per user in the workspace.`,
			`Executions are not accounted for in the global user's
					quotas but are accounted for in the workspace's quota.`,
			`Every user in the workspace increases the pooled workspace quota by <b>50k</b> executions.`,
			`1 000 executions above the pooled workspace quota is charged 1$.`,
			`<b>Dedicated workers and database</b>`,
			`<b>SAML support</b>`,
			`<b>Priority support including an automation engineer</b>`
		]
	}
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<CenteredPage>
	{#if $userStore?.is_admin}
		<PageHeader title="Workspace Settings of {$workspaceStore}" />

		<Tabs bind:selected={tab}>
			<Tab size="md" value="users">
				<div class="flex gap-2 items-center my-1"> Users & Invites </div>
			</Tab>
			<Tab size="md" value="slack">
				<div class="flex gap-2 items-center my-1"> Slack Command </div>
			</Tab>
			{#if isCloudHosted()}
				<Tab size="md" value="premium">
					<div class="flex gap-2 items-center my-1"> Premium Plans </div>
				</Tab>
			{/if}
			<Tab size="md" value="export_delete">
				<div class="flex gap-2 items-center my-1"> Export & Delete Workspace </div>
			</Tab>
		</Tabs>
		{#if tab == 'users'}
			<PageHeader title="Members ({users?.length ?? ''})" primary={false} />

			<AddUser on:new={listUsers} />

			<div class="pt-2 pb-1">
				<input placeholder="Search users" bind:value={userFilter} class="input mt-1" />
			</div>
			<div class="overflow-auto max-h-screen mb-20">
				<TableCustom>
					<tr slot="header-row">
						<th>email</th>
						<th>username</th>
						<th
							>executions (<abbr title="past 5 weeks">5w</abbr>) <Tooltip
								>An execution is calculated as 1 for any runs of scripts + 1 for each seconds above
								the first one</Tooltip
							>
						</th>
						<th />
						<th />
						<th />
					</tr>
					<tbody slot="body">
						{#if filteredUsers}
							{#each filteredUsers.slice(0, nbDisplayed) as { email, username, is_admin, operator, usage, disabled } (email)}
								<tr class="border">
									<td>{email}</td>
									<td>{username}</td>
									<td>{usage?.executions}</td>
									<td
										><div class="flex gap-1"
											>{#if disabled}
												<Badge color="red">disabled</Badge>
											{/if}</div
										></td
									>
									<td>
										<div>
											<ToggleButtonGroup
												selected={is_admin ? 'admin' : operator ? 'operator' : 'author'}
												on:selected={async (e) => {
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
												<ToggleButton position="left" value="operator" size="xs"
													>Operator <Tooltip
														>An operator can only execute and view scripts/flows/apps from your
														workspace, and only those that he has visibility on</Tooltip
													></ToggleButton
												>
												<ToggleButton position="center" value="author" size="xs"
													>Author <Tooltip
														>An Author can execute and view scripts/flows/apps, but he can also
														create new ones</Tooltip
													></ToggleButton
												>
												<ToggleButton position="right" value="admin" size="xs">Admin</ToggleButton>
											</ToggleButtonGroup>
										</div>
									</td>
									<td>
										<div class="flex gap-1">
											<button
												class="text-blue-500"
												on:click={async () => {
													await UserService.updateUser({
														workspace: $workspaceStore ?? '',
														username,
														requestBody: {
															disabled: !disabled
														}
													})
													listUsers()
												}}>{disabled ? 'enable' : 'disable'}</button
											>
											|
											<button
												class="text-red-500"
												on:click={async () => {
													await UserService.deleteUser({
														workspace: $workspaceStore ?? '',
														username
													})
													sendUserToast('User removed')
													listUsers()
												}}>remove</button
											>
										</div>
									</td>
								</tr>
							{/each}
							{#if filteredUsers?.length > 50}
								<span class="text-xs"
									>{nbDisplayed} items out of {filteredUsers.length}
									<button class="ml-4" on:click={() => (nbDisplayed += 30)}>load 30 more</button
									></span
								>
							{/if}
						{:else}
							{#each new Array(6) as _}
								<tr class="border">
									{#each new Array(4) as _}
										<td>
											<Skeleton layout={[[2]]} />
										</td>
									{/each}
								</tr>
							{/each}
						{/if}
					</tbody>
				</TableCustom>
			</div>
			<PageHeader title="Pending Invites ({invites.length ?? ''})" primary={false}>
				<InviteUser on:new={listInvites} />
			</PageHeader>

			<div class="overflow-auto max-h-screen">
				<TableCustom>
					<tr slot="header-row">
						<th>email</th>
						<th>role</th>
						<th />
					</tr>
					<tbody slot="body">
						{#each invites as { email, is_admin, operator }}
							<tr class="border">
								<td>{email}</td>
								<td
									>{#if operator}<Badge>operator</Badge>{:else if is_admin}<Badge>admin</Badge>{/if}
								</td>
								<td>
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
										}}>cancel</button
									></td
								>
							</tr>
						{/each}
					</tbody>
				</TableCustom>
			</div>

			<div class="mt-10" />
			<PageHeader title="Auto Invite" primary={false} />
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
				<div class="text-red-400 text-sm mb-2">{domain} domain not allowed for auto-invite</div>
			{/if}
		{:else if tab == 'premium'}
			{#if isCloudHosted()}
				<div class="mt-10" />
				<PageHeader title="Premium Workspace" primary={false} />

				{#if customer_id || true}
					<div class="mt-2 mb-6">
						<Button endIcon={{ icon: faExternalLink }} href="">Customer Portal</Button>
						<p class="text-xs text-gray-600 mt-1">
							See invoices, change billing information or subscription details</p
						>
					</div>
				{/if}

				<div class="text-sm mb-4">
					{#if premium_info?.premium}Number of executions in this workspace this month: <div
							class=" inline text-2xl font-bold">{premium_info.usage ?? 0}</div
						>
					{:else}
						This workspace is <b>NOT</b> on a team plan. Users use their global free-tier quotas when
						doing executions in this workspace. Upgrade to a Team or Enterprise plan to unlock unlimited
						execution in this workspace.
					{/if}
				</div>

				<div class="my-4">
					<Alert type="info" title="What is an execution">
						The single credit-unit is called an "execution". An execution corresponds to a single
						job whose duration is less than 1s. For any additional seconds of execution, an
						additional execution is accounted for. Jobs are executed on powerful cpus. Most jobs
						will take less than 200ms to execute.
					</Alert>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					{#each Object.entries(plans) as [planTitle, planDesc]}
						<div class="box p-4 text-sm  flex flex-col h-full">
							<h2 class="mb-4">{planTitle}</h2>
							<ul class="list-disc p-4">
								{#each planDesc as item}
									<li class="mt-2">{@html item}</li>
								{/each}
							</ul>

							<div class="grow" />
							{#if planTitle == 'Team'}
								{#if plan != 'team'}
									<div class="mt-4">
										<Button href="/api/w/{$workspaceStore}/workspaces/checkout"
											>Upgrade to the Team plan</Button
										>
									</div>
								{:else}
									<div class="mx-auto text-lg font-semibold">You are on this plan</div>
								{/if}
							{:else if planTitle == 'Enterprise'}
								{#if plan != 'enterprise'}
									<div class="mt-4">
										<Button href="/api/w/{$workspaceStore}/workspaces/checkout"
											>Upgrade to the Enterprise plan</Button
										>
									</div>
								{:else}
									<div class="mx-auto text-lg font-semibold">You are on this plan</div>
								{/if}
							{:else if !plan}
								<div class="mx-auto text-lg font-semibold">You are on the free-tier plan</div>
							{:else}
								<div class="mt-4 w-full">
									<Button href="/api/w/{$workspaceStore}/workspaces/checkout"
										>Upgrade to the {planTitle} plan</Button
									>
								</div>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		{:else if tab == 'slack'}
			<PageHeader title="Slack integration" primary={false} />
			<p class="text-xs text-gray-700 my-1">
				Status: {#if team_name}Connected to slack workspace <Badge>{team_name}</Badge>{:else}Not
					connected{/if}
			</p>
			{#if team_name}
				<div class="flex flex-col gap-2 max-w-sm">
					<Button
						size="sm"
						endIcon={{ icon: faSlack }}
						btnClasses="mt-2"
						variant="border"
						on:click={async () => {
							await OauthService.disconnectSlack({
								workspace: $workspaceStore ?? ''
							})
							loadSettings()
							sendUserToast('Disconnected Slack')
						}}
					>
						Disconnect Slack
					</Button>
					<Button
						size="sm"
						endIcon={{ icon: faScroll }}
						href="/scripts/add?hub=hub%2F314%2Fslack%2Fexample_of_responding_to_a_slack_command_slack"
					>
						Create a script to handle slack commands
					</Button>
					<Button size="sm" endIcon={{ icon: faBarsStaggered }} href="/flows/add?hub=28">
						Create a flow to handle slack commands
					</Button>
				</div>
			{:else}
				<Button size="sm" endIcon={{ icon: faSlack }} href="/api/oauth/connect_slack">
					Connect to Slack
				</Button>
			{/if}
			<h3 class="mt-5 text-gray-700"
				>Script or flow to run on /windmill command <Tooltip>
					The script or flow to be triggered when the `/windmill` command is invoked. The script or
					flow chosen is passed the parameters <pre>response_url: string, text: string</pre>
					respectively the url to reply directly to the trigger and the text of the command.</Tooltip
				>
			</h3>
			<ScriptPicker
				kind={Script.kind.SCRIPT}
				allowFlow
				bind:itemKind
				bind:scriptPath
				{initialPath}
				on:select={editSlackCommand}
			/>
		{:else if tab == 'export_delete'}
			<div class="mt-10" />
			<PageHeader title="Export workspace" primary={false} />
			<div class="flex justify-start">
				<Button size="sm" href="/api/w/{$workspaceStore ?? ''}/workspaces/tarball" target="_blank">
					Export workspace as tarball
				</Button>
			</div>

			<div class="mt-20" />
			<PageHeader title="Delete workspace" primary={false} />
			<p class="italic text-xs">
				The workspace will be archived for a short period of time and then permanently deleted
			</p>
			{#if $workspaceStore === 'admins' || $workspaceStore === 'starter'}
				<p class="italic text-xs">
					This workspace cannot be deleted as it has a special function. Consult the documentation
					for more information.
				</p>
			{/if}
			<div class="flex gap-2">
				<Button
					color="red"
					disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
					size="sm"
					btnClasses="mt-2"
					on:click={async () => {
						await WorkspaceService.archiveWorkspace({ workspace: $workspaceStore ?? '' })
						sendUserToast(`Archived workspace ${$workspaceStore}`)
						workspaceStore.set(undefined)
						usersWorkspaceStore.set(undefined)
						goto('/user/workspaces')
					}}
				>
					Archive workspace
				</Button>

				{#if $superadmin}
					<Button
						color="red"
						disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
						size="sm"
						btnClasses="mt-2"
						on:click={async () => {
							await WorkspaceService.deleteWorkspace({ workspace: $workspaceStore ?? '' })
							sendUserToast(`Deleted workspace ${$workspaceStore}`)
							workspaceStore.set(undefined)
							usersWorkspaceStore.set(undefined)
							goto('/user/workspaces')
						}}
					>
						Delete workspace (superadmin)
					</Button>
				{/if}
			</div>
		{/if}
	{:else}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4" role="alert">
			<p class="font-bold">Not an admin</p>
			<p>Workspace settings are only available for admin of workspaces</p>
		</div>
	{/if}
</CenteredPage>

<style>
</style>
