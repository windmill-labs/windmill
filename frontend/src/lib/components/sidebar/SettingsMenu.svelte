<script lang="ts">
	import {
		Settings,
		User,
		ServerCog,
		Logs,
		HelpCircle,
		LogOut,
		ChevronDown,
		Building
	} from 'lucide-svelte'
	import { base } from '$app/paths'
	import { goto } from '$lib/navigation'
	import { type Item } from '$lib/utils'
	import { logout } from '$lib/logoutKit'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { USER_SETTINGS_HASH, SUPERADMIN_SETTINGS_HASH } from './settings'
	import { userWorkspaces, workspaceStore, userStore, superadmin } from '$lib/stores'

	let { isCollapsed = false }: { isCollapsed?: boolean } = $props()

	const currentWs = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	const canManageWorkspace = $derived($userStore?.is_admin || $superadmin)

	// Account / instance actions gathered under one "Settings" dropdown, shared by
	// the session rail and the global sidebar so both expose the same entry point.
	const items = $derived<Item[]>([
		...(canManageWorkspace
			? [
					{
						displayName: `${currentWs?.name ?? $workspaceStore ?? 'Workspace'} settings`,
						icon: Building,
						href: `${base}/workspace_settings`
					}
				]
			: []),
		{ displayName: 'User', icon: User, action: () => goto(USER_SETTINGS_HASH) },
		{
			displayName: 'Instance settings',
			icon: Settings,
			action: () => goto(SUPERADMIN_SETTINGS_HASH)
		},
		{ displayName: 'Workers', icon: ServerCog, href: `${base}/workers` },
		{ displayName: 'Logs', icon: Logs, href: `${base}/audit_logs` },
		{
			displayName: 'Help',
			icon: HelpCircle,
			href: 'https://www.windmill.dev/docs/intro',
			hrefTarget: '_blank'
		},
		{ displayName: 'Logout', icon: LogOut, action: () => logout(), separatorTop: true }
	])
</script>

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
