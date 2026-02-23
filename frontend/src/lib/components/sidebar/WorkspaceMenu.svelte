<script lang="ts">
	import {
		isPremiumStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore,
		workspaceColor,
		clearWorkspaceFromStorage
	} from '$lib/stores'
	import { isRuleActive } from '$lib/workspaceProtectionRules.svelte'
	import { Building, Plus, Settings, GitFork } from 'lucide-svelte'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import WorkspaceIcon from '$lib/components/workspace/WorkspaceIcon.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { workspaceAIClients } from '../copilot/lib'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import { buildWorkspaceHierarchy } from '$lib/utils/workspaceHierarchy'
	import { getContrastTextColor } from '$lib/utils'

	interface Props {
		isCollapsed?: boolean
		createMenu: MenubarBuilders['createMenu']
		// When used outside of the side bar, where links to workspace settings and such don't make as much sense.
		strictWorkspaceSelect?: boolean
	}

	function removePrefix(str: string, prefix: string): string {
		if (str.startsWith(prefix)) {
			return str.substring(prefix.length)
		}
		return str
	}

	let { isCollapsed = false, createMenu, strictWorkspaceSelect = false }: Props = $props()

	async function toggleSwitchWorkspace(id: string) {
		if ($workspaceStore === id) {
			return
		}
		workspaceAIClients.init(id)
		const editPages = [
			'/scripts/edit/',
			'/flows/edit/',
			'/apps/edit/',
			'/scripts/get/',
			'/flows/get/',
			'/apps/get/'
		]
		const isOnEditPage = editPages.some((editPage) => $page.route.id?.includes(editPage) ?? false)

		if (!isOnEditPage) {
			switchWorkspace(id)
			if ($page.url.searchParams.get('workspace')) {
				$page.url.searchParams.set('workspace', id)
			}
		} else {
			switchWorkspace(id)
			await goto('/')
		}
	}

	function getForkedWorkspace(workspaceId: string) {
		if (!$userWorkspaces) return undefined
		return $userWorkspaces.find((w) => w.id === workspaceId && w.parent_workspace_id != null)
	}

	function getParentWorkspace(parentId: string) {
		if (!$userWorkspaces) return undefined
		return $userWorkspaces.find((w) => w.id === parentId)
	}

	// Group workspaces into parent-child hierarchy using Svelte 5 derived and the new utility
	const groupedWorkspaces = $derived.by(() => {
		if (!$userWorkspaces) return []
		return buildWorkspaceHierarchy($userWorkspaces)
	})

	const itemClass =
		'text-primary flex flex-row gap-2 px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
</script>

<Menu {createMenu} usePointerDownOutside>
	{#snippet triggr({ trigger })}
		{@const forkedWorkspace = getForkedWorkspace($workspaceStore ?? '')}
		{@const parentWorkspace = forkedWorkspace
			? getParentWorkspace(forkedWorkspace.parent_workspace_id!)
			: null}
		{@const iconColor = getContrastTextColor($workspaceColor)}
		{#if forkedWorkspace && parentWorkspace}
			<MenuButton
				class="!text-xs"
				icon={GitFork}
				iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
				label={removePrefix($workspaceStore ?? '', 'wm-fork-')}
				sublabel={parentWorkspace?.name ? `Fork of ${parentWorkspace.name}` : undefined}
				{isCollapsed}
				color={$workspaceColor}
				{trigger}
			/>
		{:else}
			<MenuButton
				class="!text-xs"
				icon={Building}
				iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
				label={$workspaceStore ?? ''}
				{isCollapsed}
				color={$workspaceColor}
				{trigger}
			/>
		{/if}
	{/snippet}

	{#snippet children({ item })}
		<div class="divide-y" role="none">
			<div class="py-1">
				{#each groupedWorkspaces as { workspace, depth, isForked, parentName }}
					{@const isSelected = $workspaceStore === workspace.id}
					<MenuItem
						class={twMerge(
							'text-xs min-w-0 w-full overflow-hidden flex flex-col py-2 px-3',
							workspace.disabled && 'opacity-50 cursor-not-allowed',
							isSelected
								? 'cursor-default bg-surface-accent-selected'
								: workspace.disabled
									? ''
									: 'cursor-pointer hover:bg-surface-hover data-[highlighted]:bg-surface-hover'
						)}
						onClick={async () => {
							if (!workspace.disabled) {
								await toggleSwitchWorkspace(workspace.id)
							}
						}}
						{item}
					>
						<div class="flex items-center justify-between min-w-0 w-full">
							<div class="flex items-center gap-2 min-w-0" style:padding-left={`${depth * 16}px`}>
								<WorkspaceIcon workspaceColor={workspace.color} {isForked} {parentName} />
								<div class="min-w-0 flex-1">
									<div
										class={twMerge(
											'truncate text-left text-xs font-normal',
											isSelected ? 'text-accent' : 'text-primary'
										)}
										title={workspace.name}
									>
										{workspace.name}{workspace.disabled ? ' (user disabled)' : ''}
									</div>
									<div
										class={twMerge(
											'font-mono text-2xs whitespace-nowrap truncate text-left font-normal',
											isSelected ? 'text-accent/80' : 'text-secondary'
										)}
										title={workspace.id}
									>
										{workspace.id}
									</div>
								</div>
							</div>
						</div>
					</MenuItem>
				{/each}
			</div>
			{#if (isCloudHosted() || $superadmin) && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem href="{base}/user/create_workspace" class={itemClass} {item}>
						<Plus size={16} />
						Workspace
					</MenuItem>
				</div>
			{/if}
			{#if !strictWorkspaceSelect && !isCloudHosted() && !isRuleActive('DisableWorkspaceForking')}
				<div class="py-1" role="none">
					<MenuItem href="{base}/user/fork_workspace" class={itemClass} {item}>
						<GitFork size={16} />
						Fork current workspace
					</MenuItem>
				</div>
			{/if}
			{#if !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem
						href="{base}/user/workspaces"
						onClick={() => clearWorkspaceFromStorage()}
						class={itemClass}
						{item}
					>
						All workspaces
					</MenuItem>
				</div>
			{/if}
			{#if ($userStore?.is_admin || $superadmin) && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem href="{base}/workspace_settings" class={itemClass} {item}>
						<Settings size={16} />
						Workspace settings
					</MenuItem>
				</div>
			{/if}
		</div>
		{#if isCloudHosted() && !$isPremiumStore && !strictWorkspaceSelect}
			<div class="py-1" role="none">
				{#if $workspaceStore != 'demo'}
					<span class="text-secondary block w-full text-left px-4 py-2 text-xs"
						>{$workspaceUsageStore}/1000 free workspace execs</span
					>
					<div class="w-full bg-gray-200 h-1">
						<div class="bg-blue-400 h-1" style="width: {Math.min($workspaceUsageStore, 1000) / 10}%"
						></div>
					</div>
				{/if}
				{#if $userStore?.is_admin}
					<MenuItem
						class={twMerge(
							'text-secondary block font-normal w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900',
							'data-[highlighted]:bg-gray-100 data-[highlighted]:text-gray-900'
						)}
						onClick={() => {
							goto('/workspace_settings?tab=premium')
						}}
						{item}
					>
						Upgrade
					</MenuItem>
				{/if}
			</div>
		{/if}
		{#if $enterpriseLicense && !strictWorkspaceSelect}
			<MultiplayerMenu />
		{/if}
	{/snippet}
</Menu>
