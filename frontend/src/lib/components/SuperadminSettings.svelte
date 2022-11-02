<script lang="ts">
	import Fuse from 'fuse.js'

	import { UserService, SettingsService, GlobalUserInfo } from '$lib/gen'

	import TableCustom from '$lib/components/TableCustom.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Drawer, DrawerContent } from '$lib/components/common'

	let drawer: Drawer
	export function toggleDrawer() {
		drawer?.toggleDrawer()
	}
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

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Superadmin Settings" on:close={drawer.toggleDrawer}>
		<div class="text-xs pt-2 text-gray-500 ">
			Running windmill version (backend) {version}
		</div>

		<PageHeader title="All users" primary={false} />

		<div class="pb-1" />
		<InviteGlobalUser on:new={listUsers} />
		<div class="pb-1" />

		<input placeholder="Search users" bind:value={userFilter} class="input mt-1" />
		<div class="overflow-x-auto border mb-4">
			<TableCustom>
				<tr slot="header-row">
					<th>email</th>
					<th>superadmin</th>
					<th>auth</th>
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
		</div>
	</DrawerContent>
</Drawer>
