<script lang="ts">
	import Fuse from 'fuse.js'
	import { UserService, type WorkspaceInvite, WorkspaceService } from '../gen'
	import type { User } from '../gen'
	import { sendUserToast } from '../utils'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { userStore, usersWorkspaceStore, workspaceStore } from '../stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import { faSlack } from '@fortawesome/free-brands-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { goto } from '$app/navigation'
	import InviteUser from '$lib/components/InviteUser.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'

	let users: User[] = []
	let invites: WorkspaceInvite[] = []
	let filteredUsers: User[] | undefined
	let userFilter = ''
	let scriptPath: string
	let team_name: string | undefined
	let slackLoaded = false

	const fuseOptions = {
		includeScore: false,
		keys: ['username', 'email']
	}

	const fuse: Fuse<User> = new Fuse(users, fuseOptions)
	$: filteredUsers = fuse?.search(userFilter).map((value) => value.item)

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
			requestBody: { slack_command_script: scriptPath }
		})
		sendUserToast(`slack command script set to ${scriptPath}`)
	}

	async function loadSlack(): Promise<void> {
		const settings = await WorkspaceService.getSettings({ workspace: $workspaceStore! })
		team_name = settings.slack_name
		scriptPath = settings.slack_command_script ?? ''
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsers({ workspace: $workspaceStore! })
		fuse?.setCollection(users)
	}

	async function listInvites(): Promise<void> {
		invites = await WorkspaceService.listPendingInvites({ workspace: $workspaceStore! })
	}

	$: {
		if ($workspaceStore) {
			listUsers()
			listInvites()
			loadSlack()
		}
	}
</script>

<CenteredPage>
	{#if $userStore?.is_admin}
		<PageHeader title="Workspace Settings of {$workspaceStore}" />

		<PageHeader title="Members" primary={false} />

		<div class="pb-1">
			<input placeholder="Search users" bind:value={userFilter} class="input mt-1" />
		</div>
		<TableCustom>
			<tr slot="header-row">
				<th>email</th>
				<th>username</th>
				<th>role</th>
				<th />
			</tr>
			<tbody slot="body">
				{#if filteredUsers && users}
					{#each userFilter === '' ? users : filteredUsers as { email, username, is_admin }}
						<tr class="border">
							<td>{email}</td>
							<td>{username}</td>
							<td>{is_admin ? 'admin' : 'user'}</td>
							<td
								><button
									class="ml-2 text-red-500"
									on:click={async () => {
										await UserService.deleteUser({
											workspace: $workspaceStore ?? '',
											username
										})
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
										listUsers()
									}}>{is_admin ? 'demote' : 'promote'}</button
								></td
							>
						</tr>
					{/each}
				{/if}
			</tbody>
		</TableCustom>

		<PageHeader title="Pending invites" primary={false}>
			<InviteUser on:new={listInvites} />
		</PageHeader>
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
		<div class="mt-20" />
		<PageHeader title="Slack integration" primary={false} />
		<p class="text-xs text-gray-700 my-1">
			Status: {#if team_name}Connected to slack workspace {team_name}{:else}Not connected{/if}
		</p>
		<a
			class="default-button mt-2"
			rel="external"
			href="/api/w/{$workspaceStore}/oauth/connect/slack"
			>Connect to slack <Icon class="text-white mb-1" data={faSlack} scale={0.9} />
		</a>
		{#if team_name}
			<button
				class="default-button mt-2"
				on:click={async () => {
					await WorkspaceService.disconnectClient({
						workspace: $workspaceStore ?? '',
						clientName: 'slack'
					})
					loadSlack()
					sendUserToast('Disconnected slack')
				}}
				>Disconnect slack <Icon class="text-white mb-1" data={faSlack} scale={0.9} />
			</button>
		{/if}
		<h3 class="mt-5 text-gray-700">Select a script to run on /windmill command:</h3>
		<ScriptPicker bind:scriptPath on:select={editSlackCommand} />

		<p class="text-sm text-gray-700 italic mt-3">
			A script run from slack /windmill command is run as group 'slack' and hence every variables or
			resources used within it be visible to that group. The script chosen must be able to take as
			parameter `response_url: str, text: str`, respectively the url to reply directly to the
			trigger and the text of the command.
			<a href="/scripts/add?template=u/admin/triggered_from_slack_command"
				>Define your own trigger script from a compatible template</a
			>
		</p>
		<div class="mt-10" />
		<PageHeader title="Export workspace" primary={false} />
		<a
			class="default-button"
			target="_blank"
			href="/api/w/{$workspaceStore ?? ''}/workspaces/tarball">Export workspace as tarball</a
		>

		<div class="mt-20" />
		<PageHeader title="Delete workspace" primary={false} />
		<p class="italic text-xs">
			The workspace will be archived for a short period of time and then permanently deleted
		</p>
		<button
			on:click={async () => {
				await WorkspaceService.deleteWorkspace({ workspace: $workspaceStore ?? '' })
				sendUserToast(`Successfully deleted workspace ${$workspaceStore}`)
				workspaceStore.set(undefined)
				usersWorkspaceStore.set(undefined)
				goto('/user/workspaces')
			}}
			class="default-button mt-2 bg-red-500">Delete workspace</button
		>
	{:else}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4" role="alert">
			<p class="font-bold">Not an admin</p>
			<p>Workspace settings are only available for admin of workspaces</p>
		</div>
	{/if}
</CenteredPage>

<style>
</style>
