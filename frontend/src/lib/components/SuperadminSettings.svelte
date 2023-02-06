<script lang="ts">
	import { UserService, SettingsService, GlobalUserInfo } from '$lib/gen'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Badge, Drawer, DrawerContent } from '$lib/components/common'
	import { sendUserToast } from '$lib/utils'
	import SearchItems from './SearchItems.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'

	let drawer: Drawer
	let filter = ''

	export function openDrawer() {
		loadVersion()
		listUsers()
		drawer?.openDrawer?.()
	}

	export function toggleDrawer() {
		drawer?.toggleDrawer?.()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
		const index = $page.url.href.lastIndexOf('#')
		const hashRemoved = $page.url.href.slice(0, index)
		goto(hashRemoved)
	}

	let version: string | undefined
	let users: GlobalUserInfo[] = []
	let filteredUsers: GlobalUserInfo[] = []

	async function loadVersion(): Promise<void> {
		version = await SettingsService.backendVersion()
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({ perPage: 100000 })
	}
</script>

<SearchItems
	{filter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<Drawer bind:this={drawer} on:open={listUsers} size="900px">
	<DrawerContent overflow_y={false} title="Superadmin Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full">
			<div>
				<div class="text-xs pt-1 text-gray-500 ">
					Windmill {version}
				</div>

				<PageHeader title="All global users" primary={false} />

				<div class="pb-1" />
				<InviteGlobalUser on:new={listUsers} />
				<div class="pb-1" />

				<input placeholder="Search users" bind:value={filter} class="input mt-1" />
			</div>
			<div class="mt-2 overflow-auto">
				<TableCustom>
					<tr slot="header-row" class="sticky top-0 bg-white border-b">
						<th>email</th>
						<th>auth</th>
						<th>name</th>
						<th>company</th>
						<th />
						<th />
					</tr>
					<tbody slot="body" class="overflow-y-auto h-full max-h-full">
						{#if filteredUsers && users}
							{#each filteredUsers as { email, super_admin, login_type, name, company } (email)}
								<tr class="border">
									<td>{email}</td>
									<td>{login_type}</td>
									<td><span class="break-words">{name ?? ''}</span></td>
									<td><span class="break-words">{company ?? ''}</span></td>
									<td
										>{#if super_admin}<Badge>Superadmin</Badge>{/if}</td
									>
									<td>
										<div class="flex flex-row gap-x-1">
											<button
												class="text-blue-500"
												on:click={async () => {
													await UserService.globalUserUpdate({
														email,
														requestBody: {
															is_super_admin: !super_admin
														}
													})
													sendUserToast('User updated')
													listUsers()
												}}>{super_admin ? 'non-superadmin' : 'superadmin'}</button
											>
											|
											<button
												class="text-red-500"
												on:click={async () => {
													await UserService.globalUserDelete({ email })
													sendUserToast(`User ${email} removed`)
													listUsers()
												}}>remove</button
											>
										</div>
									</td>
								</tr>
							{/each}
						{/if}
					</tbody>
				</TableCustom>
			</div>
		</div>
	</DrawerContent>
</Drawer>
