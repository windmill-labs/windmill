<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { sendUserToast } from '$lib/toast'
	import { logout, logoutWithRedirect } from '$lib/logout'
	import { UserService, type WorkspaceInvite, WorkspaceService } from '$lib/gen'
	import { superadmin, usersWorkspaceStore, userWorkspaces, workspaceStore } from '$lib/stores'
	import { Button, Skeleton } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import SuperadminSettings from '$lib/components/SuperadminSettings.svelte'
	import { WindmillIcon } from '$lib/components/icons'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { USER_SETTINGS_HASH } from '$lib/components/sidebar/settings'
	import { switchWorkspace } from '$lib/storeUtils'
	import { Cog, Crown } from 'lucide-svelte'

	let invites: WorkspaceInvite[] = []
	let list_all_as_super_admin: boolean = false
	let workspaces: { id: string; name: string; username: string }[] | undefined = undefined

	let userSettings: UserSettings
	let superadminSettings: SuperadminSettings

	const rd = $page.url.searchParams.get('rd')

	$: if (userSettings && $page.url.hash === USER_SETTINGS_HASH) {
		userSettings.openDrawer()
	}

	async function loadInvites() {
		try {
			invites = await UserService.listWorkspaceInvites()
		} catch {}
	}

	async function loadWorkspaces() {
		if (!$usersWorkspaceStore) {
			try {
				usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
			} catch {}
		}
		if ($usersWorkspaceStore) {
			if (!$workspaceStore) {
				const local = localStorage.getItem('workspace')?.toString()
				switchWorkspace(local)
			}
		} else {
			const url = $page.url
			await logoutWithRedirect(url.href.replace(url.origin, ''))
		}
	}

	async function loadWorkspacesAsAdmin() {
		workspaces = (await WorkspaceService.listWorkspacesAsSuperAdmin({ perPage: 1000 })).map((x) => {
			return { ...x, username: 'superadmin' }
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

	$: adminsInstance = workspaces?.find((x) => x.id == 'admins')

	$: nonAdminWorkspaces = (workspaces ?? []).filter((x) => x.id != 'admins')
	$: noWorkspaces = $superadmin && nonAdminWorkspaces.length == 0

	loadInvites()
	loadWorkspaces()

	let loading = false
</script>

{#if $superadmin}
	<SuperadminSettings bind:this={superadminSettings} />
{/if}

<CenteredModal title="Select a workspace" subtitle="Logged in as {$usersWorkspaceStore?.email}">
	<h2 class="mb-4 inline-flex gap-2">
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
			btnClasses="w-full mt-2 mb-4 truncate"
			color="light"
			size="sm"
			on:click={async () => {
				workspaceStore.set('admins')
				loading = true
				await goto(rd ?? '/')
				loading = false
			}}
			variant="border"
			>Manage Windmill on the superadmins workspace
		</Button>
	{/if}

	{#if workspaces && $usersWorkspaceStore}
		{#if workspaces.length == 0}
			<p class="text-sm text-tertiary mt-2">
				You are not a member of any workspace yet. Accept an invitation or create your own
				workspace.
			</p>
		{/if}
		{#each nonAdminWorkspaces as workspace}
			<label class="block pb-2">
				<button
					class="block w-full mx-auto py-1 px-2 rounded-md border
				shadow-sm text-sm font-normal mt-1 hover:ring-1 hover:ring-indigo-300"
					on:click={async () => {
						workspaceStore.set(workspace.id)
						loading = true
						await goto(rd ?? '/')
						loading = false
					}}
					><span class="font-mono">{workspace.id}</span> - {workspace.name} as
					<span class="font-mono">{workspace.username}</span>
					{#if workspace['deleted']}
						<span class="text-red-500"> (archived)</span>
					{/if}
				</button>
				{#if $superadmin && workspace['deleted']}
					<Button
						size="xs"
						btnClasses="w-full mt-1"
						color="green"
						variant="border"
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

	<div class="flex flex-row-reverse pt-4">
		<Button
			size="sm"
			btnClasses={noWorkspaces ? 'animate-bounce hover:animate-none' : ''}
			color={noWorkspaces ? 'dark' : 'blue'}
			href="/user/create_workspace{rd ? `?rd=${encodeURIComponent(rd)}` : ''}"
			variant={noWorkspaces ? 'contained' : 'border'}
			>+&nbsp;Create a new workspace
		</Button>
	</div>

	<h2 class="mt-6 mb-4">Invites to join a Workspace</h2>
	{#if invites.length == 0}
		<p class="text-sm text-tertiary mt-2"> You don't have new invites at the moment. </p>
	{/if}
	{#each invites as invite}
		<div
			class="w-full mx-auto py-1 px-2 rounded-md border shadow-sm
			text-sm mt-1 flex flex-row justify-between items-center"
		>
			<div class="grow">
				<span class="font-mono font-semibold">{invite.workspace_id}</span>
				{#if invite.is_admin}
					<span class="text-sm">as an admin</span>
				{:else if invite.operator}
					<span class="text-sm">as an operator</span>
				{/if}
			</div>
			<div class="flex justify-end items-center flex-col sm:flex-row gap-1">
				<a
					class="font-bold p-1"
					href="/user/accept_invite?workspace={encodeURIComponent(invite.workspace_id)}{rd
						? `&rd=${encodeURIComponent(rd)}`
						: ''}"
				>
					Accept
				</a>

				<button
					class="text-red-700 font-bold p-1"
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
	<div class="flex justify-between items-center mt-10 flex-wrap gap-2">
		{#if $superadmin}
			<Button
				variant="border"
				size="sm"
				on:click={superadminSettings.openDrawer}
				startIcon={{ icon: Crown }}
			>
				Superadmin settings
			</Button>
		{/if}
		<Button variant="border" size="sm" on:click={userSettings.openDrawer} startIcon={{ icon: Cog }}>
			User settings
		</Button>

		<Button
			variant="border"
			color="blue"
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
		<p class="text-center font-medium text-tertiary text-xs mb-10">
			Logged in as {$usersWorkspaceStore?.email}
		</p>
	</div>
</div> -->
<UserSettings bind:this={userSettings} />
