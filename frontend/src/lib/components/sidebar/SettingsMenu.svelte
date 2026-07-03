<script lang="ts">
	import {
		Settings,
		User,
		ServerCog,
		Logs,
		HelpCircle,
		LogOut,
		ChevronDown,
		Building,
		Moon,
		Sun
	} from 'lucide-svelte'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { type Item } from '$lib/utils'
	import { logout } from '$lib/logoutKit'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import MenuLink from './MenuLink.svelte'
	import { USER_SETTINGS_HASH, SUPERADMIN_SETTINGS_HASH } from './settings'
	import { userWorkspaces, workspaceStore, userStore, superadmin, devopsRole } from '$lib/stores'

	let {
		isCollapsed = false,
		// Session mode drops the workspace-settings entry (the rail's global
		// workspace context doesn't apply to a session's own forked workspace),
		// but keeps the rest of the menu available.
		hideWorkspaceSettings = false
	}: { isCollapsed?: boolean; hideWorkspaceSettings?: boolean } = $props()

	const currentWs = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const canManageWorkspace = $derived($userStore?.is_admin || $superadmin)

	let darkMode = $state(false)
	function toggleDarkMode() {
		if (!document.documentElement.classList.contains('dark')) {
			document.documentElement.classList.add('dark')
			window.localStorage.setItem('dark-mode', 'dark')
		} else {
			document.documentElement.classList.remove('dark')
			window.localStorage.setItem('dark-mode', 'light')
		}
	}

	// Account / instance actions gathered under one "Settings" dropdown, shared by
	// the session rail and the global sidebar so both expose the same entry point.
	// The dropdown opens upward, so bottom-to-top the settings read: Instance,
	// Workspace, User.
	const items = $derived<Item[]>([
		{ displayName: 'User', icon: User, action: () => goto(USER_SETTINGS_HASH) },
		...(canManageWorkspace && !hideWorkspaceSettings
			? [
					{
						displayName: `${currentWs?.name ?? $workspaceStore ?? 'Workspace'} settings`,
						icon: Building,
						href: `${base}/workspace_settings`
					}
				]
			: []),
		...($superadmin
			? [
					{
						displayName: 'Instance settings',
						icon: Settings,
						action: () => goto(SUPERADMIN_SETTINGS_HASH)
					}
				]
			: []),
		{
			displayName: 'Help',
			icon: HelpCircle,
			href: 'https://www.windmill.dev/docs/intro',
			hrefTarget: '_blank'
		},
		{
			displayName: 'Switch theme',
			icon: darkMode ? Sun : Moon,
			action: () => toggleDarkMode(),
			separatorTop: true
		},
		{ displayName: 'Logout', icon: LogOut, action: () => logout() }
	])
</script>

<!-- Workers and Logs are full pages (not account/instance actions), so they sit
     as plain links above the Settings dropdown rather than inside it. -->
<div class="flex flex-col gap-1 pb-1">
	<MenuLink
		class="!text-xs"
		label="Workers"
		href="{base}/workers"
		icon={ServerCog}
		{isCollapsed}
		aiId="sidebar-menu-link-workers"
		aiDescription="Button to navigate to workers"
	/>
	{#if $devopsRole || $userStore?.is_admin}
		<MenuLink
			class="!text-xs"
			label="Logs"
			href="{base}/audit_logs"
			icon={Logs}
			{isCollapsed}
			aiId="sidebar-menu-link-logs"
			aiDescription="Button to navigate to audit logs"
		/>
	{/if}
</div>

<DropdownV2 {items} placement="top-start" class="w-full">
	{#snippet buttonReplacement()}
		<span
			class="flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-secondary text-xs hover:bg-surface-hover cursor-pointer {isCollapsed
				? 'justify-center'
				: ''}"
		>
			<Settings size={16} />
			{#if !isCollapsed}
				Settings
				<ChevronDown size={14} class="ml-auto flex-shrink-0 text-tertiary" />
			{/if}
		</span>
	{/snippet}
</DropdownV2>

<DarkModeObserver bind:darkMode />
