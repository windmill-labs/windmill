<script lang="ts">
	import {
		isPremiumStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore,
		workspaceColor
	} from '$lib/stores'
	import { Building, Plus, Settings, GitBranch, GitFork } from 'lucide-svelte'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import { WorkspaceService, type EphemeralWorkspaceInfo } from '$lib/gen'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { workspaceAIClients } from '../copilot/lib'
	import { twMerge } from 'tailwind-merge'
	import type { MenubarBuilders } from '@melt-ui/svelte'

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

	// Fetch ephemeral workspace data
	let ephemeralWorkspaces: EphemeralWorkspaceInfo[] = $state([])

	// Load ephemeral workspaces when component mounts or userWorkspaces change
	$effect(() => {
		if ($userWorkspaces) {
			WorkspaceService.listEphemeralWorkspaces()
				.then((data) => {
					ephemeralWorkspaces = data
				})
				.catch(() => {
					ephemeralWorkspaces = []
				})
		}
	})

	// Helper function to check if a workspace is ephemeral
	function isEphemeralWorkspace(workspaceId: string): boolean {
		return ephemeralWorkspaces.some((e) => e.ephemeral_workspace_id === workspaceId)
	}

	function getEphemeralWorkspace(workspaceId: String): EphemeralWorkspaceInfo | undefined {
		return ephemeralWorkspaces.find((e) => e.ephemeral_workspace_id == workspaceId)
	}

	// Group workspaces into parent-child hierarchy using Svelte 5 derived
	const groupedWorkspaces = $derived(() => {
		if (!$userWorkspaces) return []

		// Create ephemeral workspace lookup map
		const ephemeralMap = new Map<string, EphemeralWorkspaceInfo>()
		ephemeralWorkspaces.forEach((e) => {
			ephemeralMap.set(e.ephemeral_workspace_id, e)
		})

		// Separate normal workspaces from ephemeral ones
		const normalWorkspaces = $userWorkspaces.filter((w) => !ephemeralMap.has(w.id))
		const ephemeralWorkspacesList = $userWorkspaces.filter((w) => ephemeralMap.has(w.id))

		// Create groups: each normal workspace followed by its ephemeral children
		const groups: Array<{
			workspace: any
			isEphemeral: boolean
			parentId?: string
			parentName?: string
		}> = []

		normalWorkspaces.forEach((workspace) => {
			// Add the parent workspace
			groups.push({ workspace, isEphemeral: false })

			// Add its ephemeral children
			const children = ephemeralWorkspacesList.filter((w) => {
				const ephemeralInfo = ephemeralMap.get(w.id)
				return ephemeralInfo?.parent_workspace_id === workspace.id
			})
			children.forEach((child) => {
				const ephemeralInfo = ephemeralMap.get(child.id)!
				groups.push({
					workspace: child,
					isEphemeral: true,
					parentId: ephemeralInfo.parent_workspace_id,
					parentName: ephemeralInfo.parent_workspace_name
				})
			})
		})

		// Add orphaned ephemeral workspaces (those without a parent in the current list)
		ephemeralWorkspacesList.forEach((ephemeral) => {
			const ephemeralInfo = ephemeralMap.get(ephemeral.id)!
			const hasParent = normalWorkspaces.some((w) => w.id === ephemeralInfo.parent_workspace_id)
			if (!hasParent) {
				groups.push({
					workspace: ephemeral,
					isEphemeral: true,
					parentId: ephemeralInfo.parent_workspace_id,
					parentName: ephemeralInfo.parent_workspace_name
				})
			}
		})

		return groups
	})
</script>

<Menu {createMenu} usePointerDownOutside>
	{#snippet triggr({ trigger })}
		{@const ephemeralWorkspace = getEphemeralWorkspace($workspaceStore ?? '')}
		{#if ephemeralWorkspace}
			<MenuButton
				class="!text-xs !text-tertiary"
				icon={Building}
				label={ephemeralWorkspace.parent_workspace_id}
				{isCollapsed}
				color={$workspaceColor}
				{trigger}
			/>
		{:else}
			<MenuButton
				class="!text-xs"
				icon={Building}
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
				{#each groupedWorkspaces() as { workspace, isEphemeral, parentId, parentName }}
					<MenuItem
						class={twMerge(
							'text-xs min-w-0 w-full overflow-hidden flex flex-col py-1.5',
							$workspaceStore === workspace.id
								? 'cursor-default bg-surface-selected'
								: 'cursor-pointer hover:bg-surface-hover data-[highlighted]:bg-surface-hover'
						)}
						onClick={async () => {
							await toggleSwitchWorkspace(workspace.id)
						}}
						{item}
					>
						<div class="flex items-center justify-between min-w-0 w-full">
							<div
								class={twMerge('flex items-center gap-2 min-w-0', isEphemeral ? 'pl-6' : 'pl-4')}
							>
								{#if isEphemeral}
									<GitFork size={12} class="text-tertiary flex-shrink-0" />
								{/if}
								<div class="min-w-0 flex-1">
									<div
										class={twMerge(
											'truncate text-left text-[1.2em]',
											isEphemeral ? 'text-secondary' : 'text-primary'
										)}
									>
										{workspace.name}
									</div>
									<div
										class={twMerge(
											'font-mono text-2xs whitespace-nowrap truncate text-left',
											isEphemeral ? 'text-tertiary opacity-75' : 'text-tertiary'
										)}
									>
										{workspace.id}
									</div>
									{#if isEphemeral && parentName}
										<div class="text-tertiary text-2xs truncate">
											Fork of {parentName}
										</div>
									{/if}
								</div>
							</div>
							{#if workspace.color}
								<div
									class="w-5 h-5 mr-2 rounded border border-gray-300 dark:border-gray-600"
									style="background-color: {workspace.color}"
								></div>
							{/if}
						</div>
					</MenuItem>
				{/each}
			</div>
			{#if (isCloudHosted() || $superadmin) && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<a
						href="{base}/user/create_workspace"
						class="text-primary px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary flex flex-flow gap-2"
						role="menuitem"
						tabindex="-1"
					>
						<Plus size={16} />
						Workspace
					</a>
				</div>
			{/if}
			{#if !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem
						href="{base}/user/workspaces"
						onClick={() => {
							localStorage.removeItem('workspace')
						}}
						class={twMerge(
							'text-primary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary',
							'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
						)}
						{item}
					>
						All workspaces
					</MenuItem>
				</div>
			{/if}
			{#if ($userStore?.is_admin || $superadmin) && !strictWorkspaceSelect}
				<div class="py-1" role="none">
					<MenuItem
						href="{base}/workspace_settings"
						class={twMerge(
							'text-secondary px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary flex flex-flow gap-2',
							'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
						)}
						{item}
					>
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
{#if isEphemeralWorkspace($workspaceStore ?? '')}
	<Menu {createMenu} usePointerDownOutside>
		{#snippet triggr({ trigger })}
			<div class="pl-4">
			<MenuButton
				class="!text-xs"
				icon={GitFork}
				label={removePrefix($workspaceStore ?? '', 'wm-ephemeral-')}
				{isCollapsed}
				color={$workspaceColor}
				{trigger}
			/>
			</div>
		{/snippet}
	</Menu>
{/if}
