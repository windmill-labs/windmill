<script lang="ts">
	import Fuse from 'fuse.js'

	import { superadmin, usersWorkspaceStore } from '../../stores'

	import { UserService, SettingsService, GlobalUserInfo } from '../../gen'
	import { displayDate, sendUserToast, getToday } from '../../utils'
	import Icon from 'svelte-awesome'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import CenteredModal from './CenteredModal.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'

	let version: string | undefined
	let users: GlobalUserInfo[] = []
	let filteredUsers: GlobalUserInfo[] | undefined
	let userFilter = ''

	const fuseOptions = {
		includeScore: false,
		keys: ['email', 'name', 'company']
	}

	const fuse: Fuse<GlobalUserInfo> = new Fuse(users, fuseOptions)
	$: filteredUsers = fuse?.search(userFilter).map((value) => value.item)

	async function loadVersion(): Promise<void> {
		version = await SettingsService.backendVersion()
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({})
		fuse?.setCollection(users)
	}

	loadVersion()
	listUsers()
</script>

<CenteredModal title="Global Users Settings">
	<div class="flex flex-row justify-between">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
	</div>
	<div class="text-2xs text-gray-500 italic pb-6">
		Running windmill version (backend) {version}
	</div>

	<PageHeader title="All users" primary={false} />
	<div class="pb-1" />
	<InviteGlobalUser on:new={listUsers} />
	<div class="pb-1" />

	<input placeholder="Search users" bind:value={userFilter} class="input mt-1" />
	<TableCustom>
		<tr slot="header-row">
			<th>email</th>
			<th>superadmin</th>
			<th>login_type</th>
			<th>name</th>
			<th>company</th>
			<th />
		</tr>
		<tbody slot="body">
			{#if filteredUsers && users}
				{#each userFilter === '' ? users : filteredUsers as { email, super_admin, login_type, name, company }}
					<tr class="border">
						<td>{email}</td>
						<td>{super_admin}</td>
						<td>{login_type}</td>
						<td>{name}</td>
						<td>{company}</td>
						<td>
							<button
								class="text-blue-500"
								on:click={async () => {
									await UserService.globalUserUpdate({
										email,
										requestBody: {
											is_super_admin: !super_admin
										}
									})
									listUsers()
								}}>{super_admin ? 'demote' : 'promote'}</button
							></td
						>
					</tr>
				{/each}
			{/if}
		</tbody>
	</TableCustom>

	<div class="flex flex-row justify-between pt-4">
		<a href="/user/workspaces">&leftarrow; Back to workspaces</a>
	</div>
</CenteredModal>
