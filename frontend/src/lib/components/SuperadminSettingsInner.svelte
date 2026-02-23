<script lang="ts">
	import { UserService, type GlobalUserInfo, SettingService } from '$lib/gen'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import InviteGlobalUser from '$lib/components/InviteGlobalUser.svelte'
	import { Button } from '$lib/components/common'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import SearchItems from './SearchItems.svelte'
	import { page } from '$app/stores'
	import { goto as gotoUrl } from '$app/navigation'
	import Version from './Version.svelte'
	import Uptodate from './Uptodate.svelte'
	import InstanceSettings from './InstanceSettings.svelte'
	import { truncate } from '$lib/utils'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { ExternalLink, Pencil, UserMinus, UserPlus } from 'lucide-svelte'
	import DropdownV2 from './DropdownV2.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import ChangeInstanceUsername from './ChangeInstanceUsername.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import InstanceNameEditor from './InstanceNameEditor.svelte'
	import Toggle from './Toggle.svelte'
	import { instanceSettingsSelectedTab } from '$lib/stores'
	import { onDestroy, tick } from 'svelte'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'
	import {
		instanceSettingsNavigationGroups,
		tabToCategoryMap,
		tabToAuthSubTab,
		categoryToTabMap,
		buildSearchableSettingItems,
		type SearchableSettingItem
	} from './instanceSettings'
	import TextInput from './text_input/TextInput.svelte'
	import SettingsPageHeader from './settings/SettingsPageHeader.svelte'
	import SettingsSearchInput from './instanceSettings/SettingsSearchInput.svelte'

	let filter = $state('')

	let {
		closeDrawer,
		showHeaderInfo = true,
		yamlMode = $bindable(false),
		hasUnsavedChanges = $bindable(false)
	} = $props()

	function removeHash() {
		const index = $page.url.href.lastIndexOf('#')
		if (index === -1) return
		const hashRemoved = $page.url.href.slice(0, index)
		gotoUrl(hashRemoved)
	}

	onDestroy(() => {
		removeHash()
	})

	let users: GlobalUserInfo[] = $state([])
	let filteredUsers: GlobalUserInfo[] = $state([])
	let deleteConfirmedCallback: (() => void) | undefined = $state(undefined)
	let deleteUserEmail: string = $state('')
	let editWrappers: Record<string, HTMLDivElement> = $state({})
	let activeOnly = $state(false)

	async function listUsers(activeOnly: boolean): Promise<void> {
		users = await UserService.listUsersAsSuperAdmin({ perPage: 100000, activeOnly: activeOnly })
	}

	$effect(() => {
		listUsers(activeOnly)
	})

	let tab: string = $state('users')

	$effect(() => {
		tab = $instanceSettingsSelectedTab
	})
	$effect(() => {
		instanceSettingsSelectedTab.set(tab)
	})

	let nbDisplayed = $state(50)

	let instanceSettings: InstanceSettings | undefined = $state()

	let automateUsernameCreation = $state(true)
	async function getAutomateUsernameCreationSetting() {
		automateUsernameCreation =
			((await SettingService.getGlobal({ key: 'automate_username_creation' })) as any) ?? true
	}
	getAutomateUsernameCreationSetting()
	let automateUsernameModalOpen = $state(false)
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

	// The category name for InstanceSettings based on current sidebar tab
	let instanceSettingsCategory = $derived(tabToCategoryMap[tab] ?? 'Core')
	let authSubTab: 'sso' | 'oauth' | 'scim' = $derived(tabToAuthSubTab[tab] ?? 'sso')

	function handleNavigate(newTab: string) {
		if (newTab === tab) return
		tab = newTab
	}

	export function saveSettings() {
		return instanceSettings?.saveSettings()
	}

	export function discardAll() {
		instanceSettings?.discardAll()
	}

	export function syncBeforeDiff(): boolean {
		return instanceSettings?.syncBeforeDiff() ?? true
	}

	export function buildFullDiff(): { original: string; modified: string } {
		return instanceSettings?.buildFullDiff() ?? { original: '', modified: '' }
	}
	// --- Settings search ---
	const searchableItems = buildSearchableSettingItems()

	let scrollTimeout: ReturnType<typeof setTimeout> | undefined
	let highlightTimeout: ReturnType<typeof setTimeout> | undefined

	async function handleSearchSelect(item: SearchableSettingItem) {
		handleNavigate(item.tabId)
		if (item.settingKey) {
			clearTimeout(scrollTimeout)
			clearTimeout(highlightTimeout)
			await tick()
			// Wait for the tab content to render before scrolling
			scrollTimeout = setTimeout(() => {
				const el = document.querySelector(`[data-setting-key="${item.settingKey}"]`)
				if (el) {
					el.scrollIntoView({ behavior: 'smooth', block: 'center' })
					el.classList.add('setting-highlight')
					highlightTimeout = setTimeout(() => el.classList.remove('setting-highlight'), 2500)
				}
			}, 100)
		}
	}

	onDestroy(() => {
		clearTimeout(scrollTimeout)
		clearTimeout(highlightTimeout)
	})
</script>

<SearchItems
	{filter}
	items={users}
	bind:filteredItems={filteredUsers}
	f={(x) => x.email + ' ' + x.name + ' ' + x.company}
/>

<div class="flex flex-col h-full w-full">
	{#if showHeaderInfo}
		<div>
			<div class="flex justify-between">
				<div class="text-xs pt-1 text-secondary flex flex-col">
					<div>Windmill <Version /></div>
				</div>
				<div><Uptodate /></div></div
			>
		</div>
		{#if $workspaceStore !== 'admins'}
			<div class="flex flex-row-reverse">
				<Button
					variant="default"
					target="_blank"
					href="{base}/?workspace=admins"
					endIcon={{ icon: ExternalLink }}
				>
					Admins workspace
				</Button>
			</div>
		{/if}
	{/if}
	<div class="{showHeaderInfo ? 'pt-4' : ''} flex grow min-h-0">
		{#if !yamlMode}
			<!-- Sidebar Navigation -->
			<div class="w-52 shrink-0 h-full overflow-auto p-4 bg-surface flex flex-col">
				<SettingsSearchInput {searchableItems} onSelect={handleSearchSelect} class="mb-3" />
				<SidebarNavigation
					groups={instanceSettingsNavigationGroups}
					selectedId={tab}
					onNavigate={handleNavigate}
				/>
				{#if $workspaceStore !== 'admins'}
					<div class="mt-4 pt-2 border-t border-surface-hover">
						<a
							href="{base}/?workspace=admins"
							target="_blank"
							class="flex items-center gap-2 px-2 py-1.5 text-xs text-secondary hover:text-primary transition-colors"
						>
							<ExternalLink size={14} />
							Admins workspace
						</a>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Main Content -->
		<div class="flex-1 min-w-0 h-full">
			<div class="h-full overflow-auto bg-surface">
				<div class="h-fit px-8 py-4">
					{#if tab === 'users' && !yamlMode}
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
										variant="accent"
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

							<SettingsPageHeader
								title="Instance users ({users.length})"
								description="Manage all users across your Windmill instance."
								link="https://www.windmill.dev/docs/advanced/instance_settings#global-users"
							/>
							<div class="flex flex-row gap-2 items-center">
								<TextInput
									inputProps={{ placeholder: 'Search users' }}
									bind:value={filter}
									class="w-60"
								/><Toggle
									bind:checked={activeOnly}
									options={{
										left: 'Show active users only',
										leftTooltip:
											'An active user is a user who has performed at least one action in the last 30 days'
									}}
								/>

								<div class="flex-1"></div>
								<Popover placement="bottom-end" disableFocusTrap closeButton>
									{#snippet trigger()}
										<Button
											variant="accent"
											unifiedSize="md"
											startIcon={{ icon: UserPlus }}
											nonCaptureEvent
											wrapperClasses="w-fit shrink-0"
										>
											Add new user
										</Button>
									{/snippet}
									{#snippet content()}
										<InviteGlobalUser on:new={() => listUsers(activeOnly)} />
									{/snippet}
								</Popover>
							</div>
							<p class="text-hint text-2xs mt-2">
								{filteredUsers.length} user{filteredUsers.length !== 1 ? 's' : ''} found
							</p>
							<div class="mt-1">
								<DataTable
									shouldLoadMore={(filteredUsers?.length ?? 0) > 50}
									loadMore={50}
									on:loadMore={() => {
										nbDisplayed += 50
									}}
								>
									<Head>
										<tr>
											<Cell head first>Email</Cell>
											{#if automateUsernameCreation}
												<Cell head>Username</Cell>
											{/if}
											<Cell head>Name</Cell>
											<Cell head>Auth</Cell>
											{#if activeOnly}
												<Cell head>Kind</Cell>
											{/if}
											<Cell head>Role</Cell>
											<Cell head last>
												<span class="sr-only">Actions</span>
											</Cell>
										</tr>
									</Head>
									<tbody>
										{#if filteredUsers && users}
											{#each filteredUsers.slice(0, nbDisplayed) as { email, super_admin, devops, login_type, name, username, operator_only }, i (email)}
												<tr class={i % 2 === 0 ? 'bg-surface-tertiary' : 'bg-surface'}>
													<Cell first class="max-w-[200px]"
														><a href="mailto:{email}" title={email} class="truncate block"
															>{email}</a
														></Cell
													>
													{#if automateUsernameCreation}
														<Cell class="max-w-[150px]">
															{#if username}
																<span title={username} class="truncate block">{username}</span>
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
														</Cell>
													{/if}
													<Cell class="max-w-[150px]"
														><span title={name ?? ''} class="truncate block"
															>{truncate(name ?? '', 30)}</span
														></Cell
													>
													<Cell class="max-w-[100px]"
														><span title={login_type} class="truncate block">{login_type}</span
														></Cell
													>
													{#if activeOnly}
														<Cell>
															{#if operator_only}
																Operator only
															{:else}
																Developer
															{/if}
														</Cell>
													{/if}
													<Cell>
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
														>
															{#snippet children({ item })}
																<ToggleButton value={'user'} small label="User" {item} />
																<ToggleButton
																	value={'devops'}
																	small
																	label="Devops"
																	tooltip="Devops is a role that grants visibilty similar to that of a super admin, but without giving all rights. For example devops users can see service logs and crtical alerts. You can think of it as a 'readonly' super admin"
																	{item}
																/>
																<ToggleButton
																	value={'super_admin'}
																	small
																	label="Superadmin"
																	{item}
																/>
															{/snippet}
														</ToggleButtonGroup>
													</Cell>
													<Cell last>
														<div class="flex items-center justify-end">
															<div bind:this={editWrappers[email]} class="w-0 h-0 overflow-hidden">
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
															</div>
															<DropdownV2
																items={[
																	{
																		displayName: 'Edit',
																		icon: Pencil,
																		action: () => {
																			const btn = editWrappers[email]?.querySelector(
																				'[aria-label="Popup button"]'
																			)
																			if (btn instanceof HTMLElement) btn.click()
																		}
																	},
																	{
																		displayName: 'Remove',
																		icon: UserMinus,
																		type: 'delete',
																		action: () => {
																			deleteUserEmail = email
																			deleteConfirmedCallback = async () => {
																				await UserService.globalUserDelete({
																					email
																				})
																				sendUserToast(`User ${email} removed`)
																				listUsers(activeOnly)
																			}
																		}
																	}
																]}
															/>
														</div>
													</Cell>
												</tr>
											{/each}
										{/if}
									</tbody>
								</DataTable>
							</div>
						</div>
					{:else}
						<InstanceSettings
							bind:this={instanceSettings}
							hideTabs
							bind:yamlMode
							bind:hasUnsavedChanges
							tab={instanceSettingsCategory}
							{authSubTab}
							{closeDrawer}
							onNavigateToTab={(category) => {
								const targetTab = categoryToTabMap[category]
								if (targetTab) {
									handleNavigate(targetTab)
								}
							}}
						/>
					{/if}
				</div>
			</div>
		</div>
	</div>
</div>
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
		<span>Are you sure you want to remove <b>{deleteUserEmail}</b>?</span>
	</div>
</ConfirmationModal>
