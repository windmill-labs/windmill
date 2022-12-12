<script context="module">
	export function load() {
		return {
			stuff: { title: 'Workspace Settings' }
		}
	}
</script>

<script lang="ts">
	import {
		UserService,
		type WorkspaceInvite,
		WorkspaceService,
		OauthService,
		Script
	} from '$lib/gen'
	import type { User } from '$lib/gen'
	import { sendUserToast, msToSec } from '$lib/utils'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { superadmin, userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { faSlack } from '@fortawesome/free-brands-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { goto } from '$app/navigation'
	import InviteUser from '$lib/components/InviteUser.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { Badge, Button, Skeleton } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { faScroll, faBarsStaggered } from '@fortawesome/free-solid-svg-icons'
	import SearchItems from '$lib/components/SearchItems.svelte'

	let users: User[] | undefined = undefined
	let invites: WorkspaceInvite[] = []
	let filteredUsers: User[] | undefined = undefined
	let userFilter = ''
	let scriptPath: string
	let initialPath: string
	let team_name: string | undefined
	let auto_invite_domain: string | undefined
	let itemKind: 'flow' | 'script' = 'flow'

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
		scriptPath = (settings.slack_command_script ?? '').split('/').slice(1).join('/')
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

	$: domain = $userStore?.email.split('@')[1]

	$: {
		if ($workspaceStore) {
			getDisallowedAutoDomain()
			listUsers()
			listInvites()
			loadSettings()
		}
	}
</script>

<SearchItems
	filter={userFilter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<CenteredPage>
	{#if $userStore?.is_admin || $superadmin}
		<PageHeader title="Workspace Settings of {$workspaceStore}" />

		<PageHeader title="Members" primary={false} />

		<div class="pb-1">
			<input placeholder="Search users" bind:value={userFilter} class="input mt-1" />
		</div>
		<div class="overflow-auto max-h-screen">
			<TableCustom>
				<tr slot="header-row">
					<th>email</th>
					<th>username</th>
					<th>role</th>
					<th colspan="3">jobs &amp; flows (<abbr title="past two weeks">2w</abbr>)</th>
				</tr>
				<tbody slot="body">
					{#if filteredUsers}
						{#each filteredUsers as { email, username, is_admin, usage } (email)}
							<tr class="border">
								<td>{email}</td>
								<td>{username}</td>
								<td>{is_admin ? 'admin' : 'user'}</td>
								<td>{usage?.jobs}</td>
								<td>{usage?.flows}</td>
								<td>{msToSec(usage?.duration_ms)}s</td>
								<td class="whitespace-nowrap"
									><button
										class="ml-2 text-red-500"
										on:click={async () => {
											await UserService.deleteUser({
												workspace: $workspaceStore ?? '',
												username
											})
											sendUserToast('User removed')
											listUsers()
										}}>remove</button
									>
									-
									<button
										class="text-blue-500"
										on:click={async () => {
											await UserService.updateUser({
												workspace: $workspaceStore ?? '',
												username,
												requestBody: {
													is_admin: !is_admin
												}
											})
											sendUserToast('User updated')
											listUsers()
										}}>{is_admin ? 'demote' : 'promote'}</button
									></td
								>
							</tr>
						{/each}
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
		<PageHeader title="Pending invites" primary={false}>
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
					{#each invites as { email, is_admin }}
						<tr class="border">
							<td>{email}</td>
							<td>{is_admin ? 'admin' : 'user'}</td>
							<td>
								<button
									class="ml-2 text-red-500"
									on:click={async () => {
										await WorkspaceService.deleteInvite({
											workspace: $workspaceStore ?? '',
											requestBody: {
												email,
												is_admin
											}
										})
										listInvites()
									}}>remove</button
								></td
							>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</div>
		<div class="mt-20" />
		<PageHeader title="Auto Invite" primary={false} />
		<div class="flex gap-2">
			{#if auto_invite_domain != domain}
				<Button
					disabled={!allowedAutoDomain}
					on:click={async () => {
						await WorkspaceService.editAutoInvite({
							workspace: $workspaceStore ?? '',
							requestBody: { set: true }
						})
						loadSettings()
						listInvites()
					}}>Set auto-invite to {domain}</Button
				>
			{/if}
			{#if auto_invite_domain}
				<Button
					on:click={async () => {
						await WorkspaceService.editAutoInvite({
							workspace: $workspaceStore ?? '',
							requestBody: { set: false }
						})
						loadSettings()
						listInvites()
					}}>Unset auto-invite from {auto_invite_domain}</Button
				>
			{/if}
		</div>
		{#if !allowedAutoDomain}
			<div class="text-red-400 text-sm mb-2">{domain} domain not allowed for auto-invite</div>
		{/if}
		<div class="mt-20" />
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
				This workspace cannot be deleted as it has a special function. Consult the documentation for
				more information.
			</p>
		{/if}
		<Button
			color="red"
			disabled={$workspaceStore === 'admins' || $workspaceStore === 'starter'}
			size="sm"
			btnClasses="mt-2"
			on:click={async () => {
				await WorkspaceService.deleteWorkspace({ workspace: $workspaceStore ?? '' })
				sendUserToast(`Successfully deleted workspace ${$workspaceStore}`)
				workspaceStore.set(undefined)
				usersWorkspaceStore.set(undefined)
				goto('/user/workspaces')
			}}
		>
			Delete workspace
		</Button>
	{:else}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4" role="alert">
			<p class="font-bold">Not an admin</p>
			<p>Workspace settings are only available for admin of workspaces</p>
		</div>
	{/if}
</CenteredPage>

<style>
</style>
