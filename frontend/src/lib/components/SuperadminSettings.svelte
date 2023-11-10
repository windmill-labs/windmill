<script lang="ts">
	import { UserService, GlobalUserInfo } from '$lib/gen'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import SearchItems from './SearchItems.svelte'
	import { page } from '$app/stores'
	import { goto } from '$app/navigation'
	import Version from './Version.svelte'
	import Uptodate from './Uptodate.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import InstanceSettings from './InstanceSettings.svelte'
	import { truncate } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { userStore } from '$lib/stores'
	import { ExternalLink } from 'lucide-svelte'
	import { settingsKeys } from './instanceSettings'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'

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
	let deleteConfirmedCallback: (() => void) | undefined = undefined

	async function listUsers(): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({ perPage: 100000 })
	}

	let tab: 'users' | string = 'users'

	let nbDisplayed = 50
</script>

<SearchItems
	{filter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<Drawer bind:this={drawer} on:open={listUsers} size="1200px" on:close={removeHash}>
	<DrawerContent overflow_y={true} title="Instance Settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full">
			<div>
				<div class="flex justify-between">
					<div class="text-xs pt-1 text-tertiary flex flex-col">
						<div>Windmill <Version /></div>
					</div>
					<div><Uptodate /></div></div
				>
			</div>
			<div class="flex flex-row-reverse">
				<Button
					variant="border"
					color="dark"
					target="_blank"
					href="/?workspace=admins"
					endIcon={{ icon: ExternalLink }}
				>
					Admins workspace
				</Button>
			</div>
			<div class="pt-4 h-full">
				<Tabs bind:selected={tab}>
					<Tab value="users">Global Users</Tab>
					{#each settingsKeys as category}
						<Tab value={category}>{category}</Tab>
					{/each}
					<svelte:fragment slot="content">
						<div class="pt-4" />
						<TabContent value="users">
							<div class="h-full">
								<div class="py-2 mb-4">
									<InviteGlobalUser on:new={listUsers} />
								</div>

								<h3>All instance users</h3>
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
											<th />
											<th />
										</tr>
										<tbody slot="body" class="overflow-y-auto w-full h-full max-h-full">
											{#if filteredUsers && users}
												{#each filteredUsers.slice(0, nbDisplayed) as { email, super_admin, login_type, name } (email)}
													<tr class="border">
														<td>{email}</td>
														<td>{login_type}</td>
														<td><span class="break-words">{truncate(name ?? '', 30)}</span></td>
														<td>
															<ToggleButtonGroup
																selected={super_admin}
																on:selected={async (e) => {
																	if (email == $userStore?.email) {
																		sendUserToast('You cannot demote yourself', true)
																		listUsers()
																		return
																	}
																	await UserService.globalUserUpdate({
																		email,
																		requestBody: {
																			is_super_admin: !super_admin
																		}
																	})
																	sendUserToast('User updated')
																	listUsers()
																}}
															>
																<ToggleButton value={false} size="xs" label="User" />
																<ToggleButton value={true} size="xs" label="Superadmin" />
															</ToggleButtonGroup>
														</td>
														<td>
															<div class="flex flex-row gap-x-1">
																<button
																	class="text-red-500 whitespace-nowrap"
																	on:click={() => {
																		deleteConfirmedCallback = async () => {
																			await UserService.globalUserDelete({ email })
																			sendUserToast(`User ${email} removed`)
																			listUsers()
																		}
																	}}
																>
																	Remove
																</button>
															</div>
														</td>
													</tr>
												{/each}
											{/if}
										</tbody>
									</TableCustom>
								</div>
								{#if filteredUsers && filteredUsers?.length > 50 && nbDisplayed < filteredUsers.length}
									<span class="text-xs"
										>{nbDisplayed} users out of {filteredUsers.length}
										<button class="ml-4" on:click={() => (nbDisplayed += 50)}>load 50 more</button
										></span
									>
								{/if}
							</div>
						</TabContent>
						<TabContent value="" values={settingsKeys}>
							<div class="h-full"> <InstanceSettings hideTabs {tab} /> </div>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>
		</div></DrawerContent
	>
</Drawer>

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
