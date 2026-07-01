<script lang="ts">
	import { workspaceMenuHref } from './workspaceMenuHref'
	import {
		isPremiumStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore,
		workspaceColor,
		clearWorkspaceFromStorage,
		type UserWorkspace
	} from '$lib/stores'
	import { Building, Plus } from 'lucide-svelte'
	import { Badge } from '$lib/components/common'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import WorkspaceIcon from '$lib/components/workspace/WorkspaceIcon.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/state'
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
		const isOnEditPage = editPages.some((editPage) => page.route.id?.includes(editPage) ?? false)
		// An AI session is scoped to its (forked) workspace, so it makes no sense
		// to keep showing it after the user switches workspace — go home instead.
		const isOnSessionPage = page.route.id?.includes('/sessions') ?? false

		switchWorkspace(id)
		if (isOnEditPage || isOnSessionPage) {
			await goto('/')
		} else if (page.url.searchParams.get('workspace')) {
			page.url.searchParams.set('workspace', id)
		}
	}

	// An AI session is scoped to its (forked) workspace, so switching workspace
	// should leave for home (the link's navigation wins over onClick's
	// preventDefault; onClick still performs the switch). Pure logic +
	// new-tab/workspace-param handling lives in workspaceMenuHref (unit-tested).
	function workspaceHref(id: string): string {
		return workspaceMenuHref({
			routeId: page.route.id,
			base,
			pathname: page.url.pathname,
			searchParams: page.url.searchParams,
			id
		})
	}

	function onWorkspaceItemClick(e: MouseEvent, workspace: { id: string; disabled?: boolean }) {
		if (workspace.disabled) {
			e.preventDefault()
			return
		}
		// Let modifier-keyed clicks fall through so the browser can open in a new tab.
		if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) {
			return
		}
		e.preventDefault()
		toggleSwitchWorkspace(workspace.id)
	}

	// Family-only picker: list the workspace families (roots), never their
	// forks. Fork selection moves to WorkspaceScopeHeader/WorkspaceFamilyPicker,
	// rendered alongside this menu in the sidebar.
	//
	// strictWorkspaceSelect is used on standalone pages (e.g. svix webhook
	// creation) that render this menu with no scope header, so there forks must
	// stay directly selectable — list the full hierarchy.
	const familyWorkspaces = $derived.by(() => {
		if (!$userWorkspaces) return []
		const hierarchy = buildWorkspaceHierarchy($userWorkspaces)
		return strictWorkspaceSelect ? hierarchy : hierarchy.filter((w) => w.depth === 0)
	})

	function findRoot(id: string | undefined): UserWorkspace | undefined {
		if (!id || !$userWorkspaces) return undefined
		let current = $userWorkspaces.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = $userWorkspaces.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}

	// The active workspace's family root — shown in the trigger so a forked
	// active workspace still surfaces its family name here (the fork itself is
	// shown in the breadcrumb).
	const currentFamily = $derived(findRoot($workspaceStore ?? undefined))

	const itemClass =
		'text-primary w-full flex flex-row gap-2 px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
</script>

<Menu {createMenu} usePointerDownOutside placement="bottom-start">
	{#snippet triggr({ trigger })}
		<!-- Family header reflects the family (root) color, not the active
		     workspace's — switching into a fork must not recolor it. -->
		{@const familyColor = currentFamily?.color ?? $workspaceColor}
		{@const iconColor = getContrastTextColor(familyColor)}
		<MenuButton
			icon={Building}
			iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
			label={currentFamily?.name ?? $workspaceStore ?? ''}
			{isCollapsed}
			color={familyColor}
			showChevron
			emphasizeLabel
			{trigger}
		/>
	{/snippet}

	{#snippet children({ item })}
		<div class="divide-y" role="none">
			<div class="py-1">
				{#each familyWorkspaces as { workspace, depth, isForked, parentName }}
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
						href={workspace.disabled ? undefined : workspaceHref(workspace.id)}
						onClick={(e) => onWorkspaceItemClick(e, workspace)}
						{item}
					>
						<div class="flex items-center justify-between min-w-0 w-full">
							<div class="flex items-center gap-2 min-w-0" style:padding-left={`${depth * 16}px`}>
								<WorkspaceIcon
									workspaceColor={workspace.color}
									{isForked}
									isDevWorkspace={workspace.is_dev_workspace}
									{parentName}
								/>
								<div class="min-w-0 flex-1">
									<div class="flex items-center gap-1 min-w-0">
										<div
											class={twMerge(
												'truncate text-left text-xs font-normal',
												isSelected ? 'text-accent' : 'text-primary'
											)}
											title={workspace.name}
										>
											{workspace.name}{workspace.disabled ? ' (user disabled)' : ''}
										</div>
										{#if workspace.is_dev_workspace}
											<Badge color="dark-blue" small class="text-3xs px-1 py-0 dark:bg-surface-accent-primary text-white dark:text-white">dev</Badge>
										{/if}
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
