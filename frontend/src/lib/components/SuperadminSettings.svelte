<script lang="ts">
	import { UserService, GlobalUserInfo } from '$lib/gen'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Badge, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import SearchItems from './SearchItems.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'
	import Version from './Version.svelte'
	import Uptodate from './Uptodate.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import InstanceSettings from './InstanceSettings.svelte'

	let drawer: Drawer
	let filter = ''

	export function openDrawer() {
		listUsers()
		drawer?.openDrawer?.()
	}

	export function closeDrawer() {
		drawer?.closeDrawer()
		removeHash()
	}

	function removeHash() {
		const index = $page.url.href.lastIndexOf('#')
		if (index === -1) return
		const hashRemoved = $page.url.href.slice(0, index)
		goto(hashRemoved)
	}

	let users: GlobalUserInfo[] = []
	let filteredUsers: GlobalUserInfo[] = []

	async function listUsers(): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({ perPage: 100000 })
	}

	let tab: 'users' | 'settings' = 'users'
</script>

<SearchItems
	{filter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<Drawer bind:this={drawer} on:open={listUsers} size="1200px" on:clickAway={removeHash}>
	<DrawerContent overflow_y={true} title="Superadmin Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full">
			<div>
				<div class="flex justify-between">
					<div class="text-xs pt-1 text-tertiary flex flex-col">
						<div>Windmill <Version /></div>
					</div>
					<div><Uptodate /></div></div
				>
			</div>
			<div class="pt-4 h-full">
				<Tabs bind:selected={tab}>
					<Tab value="users">Global Users</Tab>
					<Tab value="settings">Instance Settings</Tab>
					<svelte:fragment slot="content">
						<div class="pt-4" />
						<TabContent value="users">
							<div class="h-full">
								<div class="p-2 border mb-4">
									<InviteGlobalUser on:new={listUsers} />
								</div>
								<div class="pb-1" />
								<div>
									<input placeholder="Search users" bind:value={filter} class="input mt-1" />
								</div>
								<div class="mt-2 overflow-auto">
									<TableCustom>
										<tr slot="header-row" class="sticky top-0 bg-surface border-b">
											<th>email</th>
											<th>auth</th>
											<th>name</th>
											<th>company</th>
											<th />
											<th />
										</tr>
										<tbody slot="body" class="overflow-y-auto w-full h-full max-h-full">
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
																	class="text-blue-500 whitespace-nowrap"
																	on:click={async () => {
																		await UserService.globalUserUpdate({
																			email,
																			requestBody: {
																				is_super_admin: !super_admin
																			}
																		})
																		sendUserToast('User updated')
																		listUsers()
																	}}>make {super_admin ? 'non-superadmin' : 'superadmin'}</button
																>
																|
																<button
																	class="text-red-500 whitespace-nowrap"
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
						</TabContent>
						<TabContent value="settings">
							<div class="h-full overflow-auto"> <InstanceSettings /> </div>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>
		</div></DrawerContent
	>
</Drawer>
