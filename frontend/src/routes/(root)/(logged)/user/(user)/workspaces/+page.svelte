<script lang="ts">
	import { goto } from '$lib/navigation'
	import { base } from '$app/paths'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { logout, logoutWithRedirect } from '$lib/logout'
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
	import { Crown, GitFork, Settings } from 'lucide-svelte'
	import { isCloudHosted } from '$lib/cloud'
	import { emptyString } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { refreshSuperadmin } from '$lib/refreshUser'
	import { buildWorkspaceHierarchy } from '$lib/utils/workspaceHierarchy'
	import type { UserWorkspace } from '$lib/stores'

	let invites: WorkspaceInvite[] = []
	let list_all_as_super_admin: boolean = false
	let workspaces: UserWorkspace[] | undefined = undefined
	let showAllForks: boolean = false

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

	$: adminsInstance = workspaces?.find((x) => x.id == 'admins') || $superadmin

	// Complete workspace hierarchy with all forks
	$: forkedWorkspacesHierarchy = (() => {
		if (!workspaces) return []

		// Filter out admin workspace
		const nonAdminWorkspaces = workspaces.filter((x) => x.id !== 'admins')

		return buildWorkspaceHierarchy(nonAdminWorkspaces)
	})()

	$: groupedNonAdminWorkspaces = forkedWorkspacesHierarchy
	$: noWorkspaces = $superadmin && groupedNonAdminWorkspaces.length == 0

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
</script>

{#if $superadmin}
	<SuperadminSettings bind:this={superadminSettings} />
{/if}

<CenteredModal title="Select a workspace" subtitle="Logged in as {$usersWorkspaceStore?.email}">
	<h2 class="mb-4 inline-flex gap-2 text-sm font-semibold text-emphasis">
		Workspaces{#if loading}<WindmillIcon spin="fast" />{/if}
	</h2>

	{#if $superadmin}
		<div class="flex flex-row-reverse pb-2">
			<Toggle
				bind:checked={list_all_as_super_admin}
				options={{ right: 'List all workspaces as superadmin' }}
			/>
		</div>
	{/if}

	{#if adminsInstance}
		<Button
			btnClasses="w-full mt-2 mb-4 truncate bg-surface-tertiary hover:bg-surface-secondary"
			size="sm"
			on:click={async () => {
				workspaceStore.set('admins')
				loading = true
				if (rd?.startsWith('http')) {
					window.location.href = rd
					return
				}
				await goto(rd ?? '/')
				loading = false
			}}
			variant="default"
			>Manage Windmill on the superadmins workspace
		</Button>
	{/if}

	{#if workspaces && $usersWorkspaceStore}
		{#if workspaces.length == 0}
			<p class="text-xs text-secondary mt-2">
				You are not a member of any workspace yet. Accept an invitation {#if createWorkspace}or
					create your own{/if}
				workspace.
			</p>
		{/if}
		{#each groupedNonAdminWorkspaces as { workspace, depth, isForked, parentName } (workspace.id)}
			<label class="block pb-2" style:padding-left={`${depth * 24}px`}>
				<Button
					variant="default"
					btnClasses="bg-surface-tertiary hover:bg-surface-secondary"
					disabled={workspace.disabled}
					on:click={async () => {
						if (!workspace.disabled) {
							speakFriendAndEnterWorkspace(workspace.id)
						}
					}}
				>
					{#if isForked}
						<GitFork size={12} class="text-primary mr-2 flex-shrink-0" />
					{/if}
					<span class="flex-1">
						{#if workspace.color}
							<span
								class="inline-block w-3 h-3 mr-2 rounded-full border border-gray-400"
								style="background-color: {workspace.color}"
							></span>
						{/if}
						<span class="font-mono" class:text-secondary={isForked}>{workspace.id}</span> -
						<span class:text-secondary={isForked}>{workspace.name}</span>
						as
						<span class="font-mono" class:text-secondary={isForked}>{workspace.username}</span>
						{#if workspace['deleted']}
							<span class="text-red-500"> (archived)</span>
						{/if}
						{#if workspace.disabled}
							<span class="text-red-500"> (user disabled in this workspace)</span>
						{/if}
						{#if isForked && parentName}
							<span class="text-secondary text-2xs mt-1">
								<br />
								Fork of {parentName}
							</span>
						{/if}
					</span>
				</Button>
				{#if $superadmin && workspace['deleted']}
					<Button
						size="xs"
						btnClasses="w-full mt-1"
						variant="default"
						on:click={async () => {
							await WorkspaceService.unarchiveWorkspace({ workspace: workspace.id })
							loadWorkspacesAsAdmin()
						}}
					>
						Unarchive {workspace.id}
					</Button>
				{/if}
			</label>
		{/each}
	{:else}
		{#each new Array(3) as _}
			<Skeleton layout={[[2], 0.5]} />
		{/each}
	{/if}

	{#if createWorkspace}
		<div class="flex flex-row-reverse pt-4">
			<Button
				size="sm"
				btnClasses={noWorkspaces ? 'animate-bounce hover:animate-none' : ''}
				href="{base}/user/create_workspace{rd ? `?rd=${encodeURIComponent(rd)}` : ''}"
				variant={noWorkspaces ? 'accent' : 'default'}
				>+&nbsp;Create a new workspace
			</Button>
		</div>
	{/if}

	{@const nonForkInvites = invites.filter((invite) => invite.parent_workspace_id == undefined)}

	<h2 class="mt-6 mb-4 text-sm font-semibold text-emphasis">Invites to join a Workspace</h2>
	{#if nonForkInvites.length == 0}
		<p class="text-xs text-secondary mt-2"> You don't have new invites at the moment. </p>
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

	{#if workspaces}
		<div class="flex flex-row pt-6 pb-2">
			<Toggle size="xs" bind:checked={showAllForks} options={{ right: 'Show workspace forks' }} />
		</div>
	{/if}

	{#if showAllForks}
		{@const allWorkspacesList = workspaces || []}
		{@const filteredInvites = invites.filter((invite) => invite.parent_workspace_id)}

		<h2 class="mb-4 text-sm font-semibold text-emphasis">Forks of the workspaces you're in</h2>
		{#if filteredInvites.length == 0}
			<p class="text-xs text-secondary mt-2"> There isn't anything here </p>
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
					<a
						class="font-semibold text-xs p-1"
						href="{base}/user/accept_invite?workspace={encodeURIComponent(invite.workspace_id)}{rd
							? `&rd=${encodeURIComponent(rd)}`
							: ''}"
					>
						Accept
					</a>

					<button
						class="text-red-700 font-semibold text-xs p-1"
						on:click={async () => {
							await UserService.declineInvite({
								requestBody: { workspace_id: invite.workspace_id }
							})
							sendUserToast(`Declined invite to ${invite.workspace_id}`)
							loadInvites()
						}}
					>
						Decline
					</button>
				</div>
			</div>
		{/each}
	{/if}

	<div class="flex justify-between items-center mt-10 flex-wrap gap-2">
		{#if $superadmin}
			<Button
				variant="default"
				size="sm"
				on:click={superadminSettings.openDrawer}
				startIcon={{ icon: Crown }}
			>
				Superadmin settings
			</Button>
		{/if}
		<Button
			variant="default"
			size="sm"
			on:click={() => userSettings.openDrawer()}
			startIcon={{ icon: Settings }}
		>
			User settings
		</Button>

		<Button
			variant="default"
			size="sm"
			on:click={async () => {
				logout()
			}}
		>
			Log out
		</Button>
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
<UserSettings bind:this={userSettings} showMcpMode={true} />
