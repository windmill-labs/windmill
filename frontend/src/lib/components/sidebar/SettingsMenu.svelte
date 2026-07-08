<script lang="ts">
	import {
		Settings,
		User,
		ServerCog,
		Logs,
		Eye,
		AlertCircle,
		HelpCircle,
		LogOut,
		ChevronDown,
		Building,
		Moon,
		Sun,
		GraduationCap,
		BookOpen,
		Github,
		Newspaper,
		Crown,
		Gauge,
		Trash2
	} from 'lucide-svelte'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { type Item } from '$lib/utils'
	import { logout } from '$lib/logoutKit'
	import { leaveCurrentWorkspace } from './leaveWorkspace'
	import { isCloudHosted } from '$lib/cloud'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import DeleteForkedWorkspaceModal from './DeleteForkedWorkspaceModal.svelte'
	import { workspaceIsFork } from '$lib/utils/workspaceHierarchy'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import DiscordIcon from '../icons/brands/Discord.svelte'
	import MenuLink from './MenuLink.svelte'
	import SideBarNotification from './SideBarNotification.svelte'
	import { markChangelogsOpened, readRecentChangelogs } from './changelogs'
	import { USER_SETTINGS_HASH, SUPERADMIN_SETTINGS_HASH } from './settings'
	import {
		userWorkspaces,
		workspaceStore,
		userStore,
		superadmin,
		devopsRole,
		enterpriseLicense,
		isCriticalAlertsUIOpen,
		usageStore,
		isPremiumStore
	} from '$lib/stores'

	let {
		isCollapsed = false,
		// Session mode: workspace the workspace-settings entry should target —
		// the session's own (possibly forked) workspace, which can differ from
		// the rail's active workspace, so the entry's href carries it as the
		// `workspace` query param. Also hides "Leave workspace" (leaving a
		// workspace from the session rail makes no sense). Unset = navigation
		// mode, entry targets the active workspace.
		workspaceSettingsTarget = undefined,
		numUnacknowledgedCriticalAlerts = 0,
		// Overridable so the kitchen-sink page can preview the cloud variant
		// (isCloudHosted reads window.location, which a dev page can't change).
		cloudHosted = isCloudHosted()
	}: {
		isCollapsed?: boolean
		workspaceSettingsTarget?: string
		numUnacknowledgedCriticalAlerts?: number
		cloudHosted?: boolean
	} = $props()

	const currentWs = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const canManageWorkspace = $derived($userStore?.is_admin || $superadmin)
	// Fork/dev workspaces are detected by their parent link, not the `wm-fork-` id prefix.
	const currentWsIsFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces ?? []))

	const settingsTargetWs = $derived(
		workspaceSettingsTarget
			? $userWorkspaces?.find((w) => w.id === workspaceSettingsTarget)
			: undefined
	)

	let leaveWorkspaceModal = $state(false)
	let deleteForkModal = $state<DeleteForkedWorkspaceModal>()

	const { recent: recentChangelogs, hasNew } = readRecentChangelogs()
	let hasNewChangelogs = $state(hasNew)
	function onChangelogsOpened() {
		markChangelogsOpened()
		hasNewChangelogs = false
	}

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

	const helpItems: Item[] = [
		{ displayName: 'Tutorials', icon: GraduationCap, href: `${base}/tutorials` },
		{
			displayName: 'Docs',
			icon: BookOpen,
			href: 'https://www.windmill.dev/docs/intro/',
			hrefTarget: '_blank'
		},
		{
			displayName: 'Feedbacks',
			icon: DiscordIcon,
			href: 'https://discord.gg/V7PM2YHsPB',
			hrefTarget: '_blank'
		},
		{
			displayName: 'Issues',
			icon: Github,
			href: 'https://github.com/windmill-labs/windmill/issues/new',
			hrefTarget: '_blank'
		},
		{
			displayName: 'Changelog',
			icon: Newspaper,
			href: 'https://www.windmill.dev/changelog/',
			hrefTarget: '_blank'
		},
		...recentChangelogs.map((changelog, i) => ({
			displayName: changelog.label,
			href: changelog.href,
			hrefTarget: '_blank' as const,
			separatorTop: i === 0
		}))
	]

	// Account / instance actions gathered under one "Settings" dropdown, shared by
	// the session rail and the global sidebar so both expose the same entry point.
	// User sits just above Logout so the account-scoped entries read as one block.
	const items = $derived<Item[]>([
		...(canManageWorkspace
			? [
					workspaceSettingsTarget
						? {
								displayName: `${settingsTargetWs?.name ?? workspaceSettingsTarget} settings`,
								icon: Building,
								// The `workspace` query param switches the app to the target
								// workspace on arrival (full loads and client-side navs alike).
								href: `${base}/workspace_settings?workspace=${workspaceSettingsTarget}`
							}
						: {
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
		...(!canManageWorkspace && !workspaceSettingsTarget
			? [
					{
						displayName: 'Leave workspace',
						icon: LogOut,
						type: 'delete' as const,
						action: () => (leaveWorkspaceModal = true)
					}
				]
			: []),
		// Fork deletion is a global-sidebar action on the active workspace, so keep it
		// out of the session rail's per-target settings entry (`workspaceSettingsTarget`).
		...(currentWsIsFork && !workspaceSettingsTarget
			? [
					{
						displayName: 'Delete forked workspace',
						icon: Trash2,
						type: 'delete' as const,
						action: () => deleteForkModal?.openModal()
					}
				]
			: []),
		{
			displayName: 'Help',
			icon: HelpCircle,
			submenuItems: helpItems,
			extra: helpPing,
			separatorTop: true
		},
		{
			displayName: 'Switch theme',
			icon: darkMode ? Sun : Moon,
			action: () => toggleDarkMode()
		},
		{
			// The email is the label itself; the crown icon carries the admin role.
			displayName: $userStore?.non_member
				? `${$userStore?.email} (superadmin, not a member)`
				: ($userStore?.email ?? 'User'),
			icon: $userStore?.is_admin || $userStore?.non_member ? Crown : User,
			submenuItems: [
				{
					displayName: 'Account settings',
					icon: Settings,
					action: () => goto(USER_SETTINGS_HASH)
				},
				...(cloudHosted && !$isPremiumStore
					? [
							{
								displayName: `${$usageStore}/1000 user execs`,
								icon: Gauge,
								disabled: true
							}
						]
					: [])
			]
		},
		{ displayName: 'Logout', icon: LogOut, action: () => logout() }
	])

	const logsItems = $derived<Item[]>([
		{ displayName: 'Audit logs', icon: Eye, href: `${base}/audit_logs` },
		...($devopsRole
			? [{ displayName: 'Service logs', icon: Logs, href: `${base}/service_logs` }]
			: []),
		...($enterpriseLicense
			? [
					{
						displayName: 'Critical alerts',
						icon: AlertCircle,
						action: () => isCriticalAlertsUIOpen.set(true),
						extra: criticalAlertsBadge
					}
				]
			: [])
	])
</script>

{#snippet criticalAlertsBadge()}
	{#if numUnacknowledgedCriticalAlerts > 0}
		<SideBarNotification notificationCount={numUnacknowledgedCriticalAlerts} />
	{/if}
{/snippet}

{#snippet helpPing()}
	{#if hasNewChangelogs}
		<span class="relative flex h-2 w-2">
			<span
				class="animate-ping absolute inline-flex h-full w-full rounded-full bg-frost-400 opacity-75"
			></span>
			<span class="relative inline-flex rounded-full h-2 w-2 bg-frost-500"></span>
		</span>
	{/if}
{/snippet}

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
		<DropdownV2
			items={logsItems}
			placement="top-start"
			class="w-full"
			enableFlyTransition
			aiId="sidebar-menu-link-logs"
			aiDescription="Button to open the logs menu (audit logs, service logs, critical alerts)"
		>
			{#snippet buttonReplacement()}
				<span
					class="relative flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-secondary text-xs hover:bg-surface-hover cursor-pointer {isCollapsed
						? 'justify-center'
						: ''}"
				>
					<Logs size={16} />
					{#if !isCollapsed}
						Logs
						<span class="ml-auto flex items-center gap-2">
							{#if numUnacknowledgedCriticalAlerts > 0}
								<SideBarNotification notificationCount={numUnacknowledgedCriticalAlerts} />
							{/if}
							<ChevronDown size={14} class="flex-shrink-0 text-tertiary" />
						</span>
					{:else if numUnacknowledgedCriticalAlerts > 0}
						<span class="absolute top-0.5 right-0.5">
							<SideBarNotification
								notificationCount={numUnacknowledgedCriticalAlerts}
								small={true}
							/>
						</span>
					{/if}
				</span>
			{/snippet}
		</DropdownV2>
	{:else}
		<MenuLink
			class="!text-xs"
			label="Audit logs"
			href="{base}/audit_logs"
			icon={Eye}
			disabled={$userStore?.operator}
			{isCollapsed}
			aiId="sidebar-menu-link-audit-logs"
			aiDescription="Button to navigate to audit logs"
		/>
	{/if}
</div>

<!-- Changelogs count as seen once the menu closes (not on open), so the Help-row
     dot stays visible while the user is looking at the open menu. -->
<DropdownV2
	{items}
	placement="top-start"
	class="w-full"
	enableFlyTransition
	on:close={onChangelogsOpened}
>
	{#snippet buttonReplacement()}
		<span
			class="relative flex items-center gap-2 w-full px-2 py-1.5 rounded-md text-secondary text-xs hover:bg-surface-hover cursor-pointer {isCollapsed
				? 'justify-center'
				: ''}"
		>
			<Settings size={16} />
			{#if !isCollapsed}
				Settings
				<ChevronDown size={14} class="ml-auto flex-shrink-0 text-tertiary" />
			{/if}
			{#if hasNewChangelogs}
				<span
					class="flex h-2 w-2 absolute {isCollapsed
						? 'top-0.5 right-0.5'
						: 'right-7 top-1/2 -translate-y-1/2'}"
				>
					<span
						class="animate-ping absolute inline-flex h-full w-full rounded-full bg-frost-400 opacity-75"
					></span>
					<span class="relative inline-flex rounded-full h-2 w-2 bg-frost-500"></span>
				</span>
			{/if}
		</span>
	{/snippet}
</DropdownV2>

<ConfirmationModal
	open={leaveWorkspaceModal}
	title="Leave workspace"
	confirmationText="Leave workspace"
	on:canceled={() => {
		leaveWorkspaceModal = false
	}}
	on:confirmed={() => {
		leaveWorkspaceModal = false
		void leaveCurrentWorkspace()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to leave this workspace?</span>
	</div>
</ConfirmationModal>

<DeleteForkedWorkspaceModal bind:this={deleteForkModal} />

<DarkModeObserver bind:darkMode />
