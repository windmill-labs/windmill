<script lang="ts">
	import { UserService, type GlobalUserInfo, SettingService } from '$lib/gen'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Button, Drawer, DrawerContent, Tab, Tabs } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import SearchItems from './SearchItems.svelte'
	import { page } from '$app/stores'
	import { goto as gotoUrl } from '$app/navigation'
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
	import ChangeInstanceUsername from './ChangeInstanceUsername.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import InstanceNameEditor from './InstanceNameEditor.svelte'
	import Toggle from './Toggle.svelte'
	import { instanceSettingsSelectedTab } from '$lib/stores'
	let drawer: Drawer
	let filter = ''

	export function openDrawer() {
		listUsers(activeOnly)
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
		gotoUrl(hashRemoved)
	}

	let users: GlobalUserInfo[] = []
	let filteredUsers: GlobalUserInfo[] = []
	let deleteConfirmedCallback: (() => void) | undefined = undefined
	let activeOnly = false

	async function listUsers(activeOnly: boolean): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({ perPage: 100000, activeOnly: activeOnly })
	}

	$: listUsers(activeOnly)

	let tab: 'users' | string = 'users'

	$: $instanceSettingsSelectedTab, (tab = $instanceSettingsSelectedTab)
	$: tab, instanceSettingsSelectedTab.set(tab)

	let nbDisplayed = 50

	let instanceSettings

	let automateUsernameCreation = false
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? false
	}
	getAutomateUsernameCreationSetting()
	let automateUsernameModalOpen = false
	async function enableAutomateUsernameCreationSetting() {
		await SettingService.setGlobal({
			key: 'automate_username_creation',
			requestBody: { value: true }
		})
		getAutomateUsernameCreationSetting()
		sendUserToast('Automatic username creation enabled')
		listUsers(activeOnly)
	}

	async function updateName(name: string | undefined, email: string) {
		try {
			await UserService.globalUserUpdate({
				email,
				requestBody: {
					name
				}
			})
			sendUserToast('User updated')
			listUsers(activeOnly)
		} catch (e) {
			sendUserToast('Error updating user', true)
		}
	}
</script>

<SearchItems
	{filter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<Drawer
	bind:this={drawer}
	on:open={() => listUsers(activeOnly)}
	size="1200px"
	on:close={removeHash}
>
	<DrawerContent overflow_y={true} title="Instance settings" on:close={closeDrawer}>
		<div class="flex flex-col h-full w-full">
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
					href="{base}/?workspace=admins"
					endIcon={{ icon: ExternalLink }}
				>
					Admins workspace
				</Button>
			</div>
			<div class="pt-4 h-full">
				<Tabs bind:selected={tab}>
					<Tab value="users">Users</Tab>
					{#each settingsKeys as category}
						<Tab value={category}>{category}</Tab>
					{/each}
					<svelte:fragment slot="content">
						<div class="pt-4"></div>
						<TabContent value="users">
							<div class="h-full">
								{#if !automateUsernameCreation && !isCloudHosted()}
									<div class="mb-4">
										<h3 class="mb-2"> Automatic username creation </h3>
										<div class="mb-2">
											<span class="text-primary text-sm"
												>Automatically create a username for new users based on their email, shared
												across workspaces. <a
													target="_blank"
													href="https://www.windmill.dev/docs/advanced/instance_settings#automatic-username-creation"
													>Learn more</a
												></span
											>
										</div>
										<Button
											btnClasses="w-auto"
											size="sm"
											color="dark"
											on:click={() => {
												automateUsernameModalOpen = true
											}}
										>
											Enable (recommended)
										</Button>
										<ConfirmationModal
											open={automateUsernameModalOpen}
											on:confirmed={() => {
												automateUsernameModalOpen = false
												enableAutomateUsernameCreationSetting()
											}}
											on:canceled={() => (automateUsernameModalOpen = false)}
											title="Automatic username creation"
											confirmationText="Enable"
										>
											Once activated, it will not be possible to disable this feature. In case
											existing users have different usernames in different workspaces, you will have
											to manually confirm the username for each user.
										</ConfirmationModal>
									</div>
								{/if}

								<div class="py-2 mb-4">
									<InviteGlobalUser on:new={() => listUsers(activeOnly)} />
								</div>

								<div class="flex flex-row justify-between">
									<h3>All instance users</h3>
									<Toggle
										bind:checked={activeOnly}
										options={{
											left: 'Show active users only',
											leftTooltip:
												'An active user is a user who has performed at least one action in the last 30 days'
										}}
									/>
								</div>
								<div class="pb-1"></div>
								<div>
									<input placeholder="Search users" bind:value={filter} class="input mt-1" />
								</div>
								<div class="mt-2 overflow-auto">
									<TableCustom>
										<tr slot="header-row" class="sticky top-0 bg-surface border-b">
											<th>email</th>
											<th>auth</th>
											<th>name</th>
											{#if automateUsernameCreation}
												<th>username</th>
											{/if}
											{#if activeOnly}
												<th>kind</th>
											{/if}
											<th></th>
											<th></th>
										</tr>
										<tbody slot="body" class="overflow-y-auto w-full h-full max-h-full">
											{#if filteredUsers && users}
												{#each filteredUsers.slice(0, nbDisplayed) as { email, super_admin, devops, login_type, name, username, operator_only } (email)}
													<tr class="border">
														<td>{email}</td>
														<td>{login_type}</td>
														<td><span class="break-words">{truncate(name ?? '', 30)}</span></td>

														{#if automateUsernameCreation}
															<td>
																{#if username}
																	{username}
																{:else}
																	{#key filteredUsers.map((u) => u.username).join()}
																		<ChangeInstanceUsername
																			username=""
																			{email}
																			isConflict
																			on:renamed={() => {
																				listUsers(activeOnly)
																			}}
																		/>
																	{/key}
																{/if}
															</td>
														{/if}
														{#if activeOnly}
															<td>
																{#if operator_only}
																	Operator only
																{:else}
																	Developer
																{/if}
															</td>
														{/if}
														<td>
															<ToggleButtonGroup
																selected={super_admin ? 'super_admin' : devops ? 'devops' : 'user'}
																on:selected={async (e) => {
																	if (email == $userStore?.email) {
																		sendUserToast('You cannot demote yourself', true)
																		listUsers(activeOnly)
																		return
																	}

																	let role = e.detail

																	if (role === 'super_admin') {
																		await UserService.globalUserUpdate({
																			email,
																			requestBody: {
																				is_super_admin: true,
																				is_devops: false
																			}
																		})
																	}
																	if (role === 'devops') {
																		await UserService.globalUserUpdate({
																			email,
																			requestBody: {
																				is_super_admin: false,
																				is_devops: true
																			}
																		})
																	}
																	if (role === 'user') {
																		await UserService.globalUserUpdate({
																			email,
																			requestBody: {
																				is_super_admin: false,
																				is_devops: false
																			}
																		})
																	}
																	sendUserToast('User updated')
																	listUsers(activeOnly)
																}}
																let:item
															>
																<ToggleButton value={'user'} size="xs" label="User" {item} />
																<ToggleButton
																	value={'devops'}
																	size="xs"
																	label="Devops"
																	tooltip="Devops is a role that grants visibilty similar to that of a super admin, but without giving all rights. For example devops users can see service logs and crtical alerts. You can think of it as a 'readonly' super admin"
																	{item}
																/>
																<ToggleButton
																	value={'super_admin'}
																	size="xs"
																	label="Superadmin"
																	{item}
																/>
															</ToggleButtonGroup>
														</td>
														<td>
															<div class="flex flex-row gap-x-1 justify-end">
																<InstanceNameEditor
																	{login_type}
																	value={name}
																	{username}
																	{email}
																	on:refresh={() => {
																		listUsers(activeOnly)
																	}}
																	on:save={(e) => {
																		updateName(e.detail, email)
																	}}
																	on:renamed={() => {
																		listUsers(activeOnly)
																	}}
																	{automateUsernameCreation}
																/>
																<Button
																	color="light"
																	variant="contained"
																	size="xs"
																	spacingSize="xs2"
																	btnClasses="text-red-500"
																	on:click={() => {
																		deleteConfirmedCallback = async () => {
																			await UserService.globalUserDelete({ email })
																			sendUserToast(`User ${email} removed`)
																			listUsers(activeOnly)
																		}
																	}}
																>
																	Remove
																</Button>
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
							<InstanceSettings
								bind:this={instanceSettings}
								hideTabs
								hideSave
								{tab}
								{closeDrawer}
							/>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>
		</div>
		{#if tab != 'users'}
			<div class="absolute bottom-2 w-[95%]">
				<Button
					color="dark"
					on:click={() => {
						instanceSettings?.saveSettings()
					}}
				>
					Save
				</Button>
			</div>{/if}
	</DrawerContent>
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
