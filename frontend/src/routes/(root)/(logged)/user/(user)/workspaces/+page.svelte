<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$app/paths'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { logout, logoutWithRedirect } from '$lib/logoutKit'
	import { UserService, type WorkspaceInvite, WorkspaceService } from '$lib/gen'
	import {
		superadmin,
		usersWorkspaceStore,
		userWorkspaces,
		workspaceStore,
		userStore,
		getWorkspaceFromStorage
	} from '$lib/stores'
	import { Button, Skeleton } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import SuperadminSettings from '$lib/components/SuperadminSettings.svelte'
	import { WindmillIcon } from '$lib/components/icons'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { USER_SETTINGS_HASH } from '$lib/components/sidebar/settings'
	import { switchWorkspace } from '$lib/storeUtils'
	import { GitFork, Settings, User, Search, ChevronsDownUp, ChevronsUpDown } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'
	import { emptyString } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { refreshSuperadmin } from '$lib/refreshUser'
	import WorkspaceTreeView from '$lib/components/workspace/WorkspaceTreeView.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import type { UserWorkspace } from '$lib/stores'

	let invites: WorkspaceInvite[] = []
	let list_all_as_super_admin: boolean = false
	let workspaces: UserWorkspace[] | undefined = undefined
	let showAllForks: boolean = false

	// Workspace tree controls
	let workspaceSearchFilter = ''
	let workspaceAllExpanded = false
	let workspaceHasForks = false
	let workspaceTreeView: WorkspaceTreeView | undefined = undefined

	let userSettings: UserSettings
	let superadminSettings: SuperadminSettings

	$: rd = $page.url.searchParams.get('rd')

	$: if (userSettings && $page.url.hash.startsWith(USER_SETTINGS_HASH)) {
		const mcpMode = $page.url.hash.includes('-mcp')
		userSettings.openDrawer(mcpMode)
	}

	async function loadInvites() {
		try {
			invites = await UserService.listWorkspaceInvites()
		} catch {}
	}

	async function loadWorkspaces() {
		console.log('loading workspaces', $usersWorkspaceStore)
		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			} catch {}
		}
		if ($usersWorkspaceStore) {
			if (!$workspaceStore) {
				const local = getWorkspaceFromStorage()?.toString()
				switchWorkspace(local)
			}
		} else {
			const url = $page.url
			console.log('logout 1')
			await logoutWithRedirect(url.href.replace(url.origin, ''))
		}
	}

	async function loadWorkspacesAsAdmin() {
		workspaces = (await WorkspaceService.listWorkspacesAsSuperAdmin({ perPage: 1000 })).map((x) => {
			return { ...x, username: 'superadmin', disabled: false }
		})
	}

	function handleListWorkspaces() {
		if (list_all_as_super_admin) {
			loadWorkspacesAsAdmin()
		} else {
			workspaces = $userWorkspaces
		}
	}
	$: list_all_as_super_admin != undefined && $userWorkspaces && handleListWorkspaces()

	$: allWorkspaces = workspaces || []
	$: noWorkspaces = $superadmin && allWorkspaces.length == 0
	$: onlyAdminsWorkspace = allWorkspaces.length === 1 && allWorkspaces[0].id === 'admins'

	async function getCreateWorkspaceRequireSuperadmin() {
		const r = await fetch(base + '/api/workspaces/create_workspace_require_superadmin')
		const t = await r.text()
		createWorkspace = t != 'true'
	}

	let createWorkspace = $superadmin || isCloudHosted()

	$: if ($superadmin) {
		createWorkspace = true
	}

	if (!createWorkspace) {
		getCreateWorkspaceRequireSuperadmin()
	}

	refreshSuperadmin()
	loadInvites()
	loadWorkspaces()

	let loading = false

	async function speakFriendAndEnterWorkspace(workspaceId: string) {
		loading = true

		// Special handling for admins workspace
		if (workspaceId === 'admins') {
			workspaceStore.set('admins')
			if (rd?.startsWith('http')) {
				window.location.href = rd
				return
			}
			await goto(rd ?? '/')
			loading = false
			return
		}

		// Regular workspace handling
		workspaceStore.set(undefined)
		workspaceStore.set(workspaceId)
		$userStore = await getUserExt($workspaceStore!)
		if (!$userStore?.is_super_admin && $userStore?.operator) {
			let defaultApp = await WorkspaceService.getWorkspaceDefaultApp({
				workspace: $workspaceStore!
			})
			if (!emptyString(defaultApp.default_app_path)) {
				await goto(`/apps/get/${defaultApp.default_app_path}`)
			} else {
				if (rd?.startsWith('http')) {
					window.location.href = rd
					return
				}
				await goto(rd ?? '/')
			}
		} else {
			try {
				if (rd?.startsWith('http')) {
					window.location.href = rd
				} else {
					await goto(rd ?? '/')
				}
				console.log('Workspace selected going to ' + (rd ? `rd: ${rd}` : 'home'))
			} catch (e) {
				console.error('Error going to', rd, e)
				window.location.reload()
			}
		}
		loading = false
	}

	function workspaceExpandCollapseAll() {
		workspaceTreeView?.handleExpandCollapseAll()
	}
</script>

{#if $superadmin}
	<SuperadminSettings bind:this={superadminSettings} disableChatOffset />
{/if}

<CenteredModal
	title="Select a workspace"
	subtitle="Logged in as {$usersWorkspaceStore?.email}"
	centerVertically={false}
>
	{@const nonForkInvites = invites.filter((invite) => invite.parent_workspace_id == undefined)}
	<div class="flex flex-col">
		<div class="flex flex-row items-center gap-2 justify-between mb-4">
			<h2 class="inline-flex gap-2 text-sm font-semibold text-emphasis flex-shrink-0">
				Workspaces{#if loading}<WindmillIcon spin="fast" />{/if}
			</h2>

			{#if allWorkspaces.length > 1}
				<div class="flex gap-2 items-center">
					<div class="relative text-primary flex-1 max-w-48">
						<TextInput
							inputProps={{
								placeholder: 'Search workspaces...'
							}}
							size="sm"
							bind:value={workspaceSearchFilter}
							class="!pr-8"
						/>
						<Search size={14} class="text-secondary absolute right-2 top-0 mt-2" />
					</div>
					{#if workspaceHasForks}
						<Button
							onClick={() => workspaceExpandCollapseAll?.()}
							title={workspaceAllExpanded ? 'Collapse all' : 'Expand all'}
							startIcon={{ icon: workspaceAllExpanded ? ChevronsDownUp : ChevronsUpDown }}
							size="xs2"
							variant="default"
						>
							{workspaceAllExpanded ? 'Collapse' : 'Expand'}
						</Button>
					{/if}
				</div>
			{/if}
		</div>

		{#if $superadmin}
			<div class="flex justify-end mb-2">
				<Toggle
					bind:checked={list_all_as_super_admin}
					options={{ right: 'List all workspaces as superadmin' }}
					size="xs"
				/>
			</div>
		{/if}

		{#if workspaces && $usersWorkspaceStore}
			{#if workspaces.length == 0}
				<p class="text-xs text-secondary mt-2">
					You are not a member of any workspace yet. Accept an invitation {#if createWorkspace}or
						create your own{/if}
					workspace.
				</p>
			{:else}
				<WorkspaceTreeView
					workspaces={allWorkspaces}
					onEnterWorkspace={speakFriendAndEnterWorkspace}
					onUnarchive={async (_workspaceId) => {
						if (list_all_as_super_admin) {
							loadWorkspacesAsAdmin()
						} else {
							loadWorkspaces()
						}
					}}
					bind:searchFilter={workspaceSearchFilter}
					bind:allExpanded={workspaceAllExpanded}
					bind:hasForks={workspaceHasForks}
					bind:this={workspaceTreeView}
				/>
			{/if}
		{:else}
			{#each new Array(3) as _, i (i)}
				<Skeleton layout={[[2], 0.5]} />
			{/each}
		{/if}

		{#if createWorkspace}
			<div class="flex flex-row-reverse pt-4 w-full">
				<AnimatedButton animate={onlyAdminsWorkspace} baseRadius="6px" animationDuration="2s" marginWidth="2px" wrapperClasses="w-full">
					<Button
						unifiedSize="sm"
						href="{base}/user/create_workspace{rd ? `?rd=${encodeURIComponent(rd)}` : ''}"
						variant={onlyAdminsWorkspace || noWorkspaces ? 'accent' : 'default'}
						wrapperClasses="w-full"
						>+&nbsp;Create a new workspace
					</Button>
				</AnimatedButton>
			</div>
		{/if}

		<div class="flex flex-row items-center justify-between mt-8">
			<h2 class="text-sm font-semibold text-emphasis">Invites to join a Workspace</h2>
			{#if workspaces}
				<Toggle size="xs" bind:checked={showAllForks} options={{ right: 'Show workspace forks' }} />
			{/if}
		</div>

		<div class="mt-4"></div>

		{#if nonForkInvites.length == 0}
			<p class="text-xs text-secondary"> You don't have new invites at the moment. </p>
		{/if}

		{#each nonForkInvites as invite}
			<div
				class="w-full mx-auto py-1 px-2 rounded-md border border-border-light
			text-xs mt-1 flex flex-row justify-between items-center"
			>
				<div class="grow">
					<span class="font-mono font-semibold text-emphasis">{invite.workspace_id}</span>
					{#if invite.is_admin}
						<span class="text-xs text-primary">as an admin</span>
					{:else if invite.operator}
						<span class="text-xs text-primary">as an operator</span>
					{/if}
				</div>
				<div class="flex justify-end items-center flex-col sm:flex-row gap-1">
					<Button
						variant="accent"
						size="xs2"
						href="{base}/user/accept_invite?workspace={encodeURIComponent(invite.workspace_id)}{rd
							? `&rd=${encodeURIComponent(rd)}`
							: ''}"
					>
						Accept
					</Button>

					<Button
						variant="subtle"
						size="xs2"
						on:click={async () => {
							await UserService.declineInvite({
								requestBody: { workspace_id: invite.workspace_id }
							})
							sendUserToast(`Declined invite to ${invite.workspace_id}`)
							loadInvites()
						}}
						destructive
					>
						Decline
					</Button>
				</div>
			</div>
		{/each}

		{#if showAllForks}
			{@const allWorkspacesList = workspaces || []}
			{@const filteredInvites = invites.filter((invite) => invite.parent_workspace_id)}

			<div class="mt-4"></div>
			{#if filteredInvites.length == 0}
				<p class="text-xs text-secondary"
					>There are no invites to join the forks of any workspace you're in.</p
				>
			{:else}
				<span class="mb-2 text-xs font-normal text-secondary"
					>Forks of the workspaces you're in</span
				>
			{/if}

			{#each filteredInvites as invite}
				{@const inviteWorkspace = allWorkspacesList.find((w) => w.id === invite.workspace_id)}
				<div
					class="w-full mx-auto py-1 px-2 rounded-md border border-border-light
			text-xs mt-1 flex flex-row justify-between items-center"
				>
					<div class="grow">
						<div class="flex items-center gap-2">
							{#if inviteWorkspace?.parent_workspace_id}
								<GitFork size={12} class="text-secondary flex-shrink-0" />
							{/if}
							<span class="font-mono font-semibold text-emphasis">{invite.workspace_id}</span>
						</div>
						{#if invite.is_admin}
							<span class="text-xs text-primary">as an admin</span>
						{:else if invite.operator}
							<span class="text-xs text-primary">as an operator</span>
						{/if}
						{#if invite.parent_workspace_id}
							<div class="text-secondary text-2xs mt-1">
								Fork of {invite.parent_workspace_id}
							</div>
						{/if}
					</div>
					<div class="flex justify-end items-center flex-col sm:flex-row gap-1">
						<Button
							variant="accent"
							unifiedSize="xs"
							href="{base}/user/accept_invite?workspace={encodeURIComponent(invite.workspace_id)}{rd
								? `&rd=${encodeURIComponent(rd)}`
								: ''}"
						>
							Accept
						</Button>

						<Button
							variant="subtle"
							unifiedSize="xs"
							destructive
							onClick={async () => {
								await UserService.declineInvite({
									requestBody: { workspace_id: invite.workspace_id }
								})
								sendUserToast(`Declined invite to ${invite.workspace_id}`)
								loadInvites()
							}}
						>
							Decline
						</Button>
					</div>
				</div>
			{/each}
		{/if}

		<div class="flex justify-between items-center mt-10 flex-wrap gap-2">
			{#if $superadmin}
				<Button
					variant="default"
					unifiedSize="md"
					on:click={superadminSettings.openDrawer}
					startIcon={{ icon: Settings }}
					dropdownItems={[
						{
							label: 'User settings',
							onClick: () => userSettings.openDrawer(),
							icon: User
						}
					]}
				>
					Instance settings
				</Button>
			{:else}
				<Button
					variant="default"
					unifiedSize="md"
					onClick={() => userSettings.openDrawer()}
					startIcon={{ icon: Settings }}
				>
					User settings
				</Button>
			{/if}

			<Button
				variant="accent"
				unifiedSize="md"
				on:click={async () => {
					logout()
				}}
			>
				Log out
			</Button>
		</div>
	</div>
</CenteredModal>
<!-- <div class="center-center min-h-screen p-4">
	<div
		class="border rounded-md shadow-md bg-white w-full max-w-[640px] p-4 sm:py-8 sm:px-10 mb-6 md:mb-20"
	>
		<h1 class="text-center mb-2">Select a workspace</h1>
		<p class="text-center font-semibold text-primary text-xs mb-10">
			Logged in as {$usersWorkspaceStore?.email}
		</p>
	</div>
</div> -->
<UserSettings bind:this={userSettings} showMcpMode={true} disableChatOffset />
