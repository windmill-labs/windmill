<script lang="ts">
	import {
		isPremiumStore,
		superadmin,
		userStore,
		userWorkspaces,
		workspaceStore,
		workspaceUsageStore
	} from '$lib/stores'
	import { Building, Plus, Settings } from 'lucide-svelte'
	import MenuButtonMelt from '$lib/components/sidebar/MenuButtonMelt.svelte'
	import { Menu } from '$lib/components/meltComponents'
	import { melt } from '@melt-ui/svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/base'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { workspaceAIClients } from '../copilot/lib'
	import { twMerge } from 'tailwind-merge'

	export let isCollapsed: boolean = false
	export let createMenu: any

	// When used outside of the side bar, where links to workspace settings and such don't make as much sense.
	export let strictWorkspaceSelect = false

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
</script>

<Menu {createMenu} let:item>
	<svelte:fragment slot="trigger" let:trigger>
		<MenuButtonMelt
			class="!text-xs"
			icon={Building}
			label={$workspaceStore ?? ''}
			{isCollapsed}
			color={$userWorkspaces.find((w) => w.id === $workspaceStore)?.color}
			{trigger}
		/>
	</svelte:fragment>

	<div class="divide-y" role="none">
		<div class="py-1">
			{#each $userWorkspaces as workspace}
				<button
					class={twMerge(
						'text-xs min-w-0 w-full overflow-hidden flex flex-col py-1.5',
						$workspaceStore === workspace.id
							? 'cursor-default bg-surface-selected'
							: 'cursor-pointer hover:bg-surface-hover data-[highlighted]:bg-surface-hover'
					)}
					on:click={async () => {
						await toggleSwitchWorkspace(workspace.id)
					}}
					use:melt={item}
				>
					<div class="flex items-center justify-between min-w-0 w-full">
						<div>
							<div class="text-primary pl-4 truncate text-left text-[1.2em]">{workspace.name}</div>
							<div
								class="text-tertiary font-mono pl-4 text-2xs whitespace-nowrap truncate text-left"
							>
								{workspace.id}
							</div>
						</div>
						{#if workspace.color}
							<div
								class="w-5 h-5 mr-2 rounded border border-gray-300 dark:border-gray-600"
								style="background-color: {workspace.color}"
							/>
						{/if}
					</div>
				</button>
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
				<a
					href="{base}/user/workspaces"
					on:click={() => {
						localStorage.removeItem('workspace')
					}}
					class={twMerge(
						'text-primary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary',
						'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
					)}
					role="menuitem"
					tabindex="-1"
					use:melt={item}
				>
					All workspaces
				</a>
			</div>
		{/if}
		{#if ($userStore?.is_admin || $superadmin) && !strictWorkspaceSelect}
			<div class="py-1" role="none">
				<a
					href="{base}/workspace_settings"
					class={twMerge(
						'text-secondary px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary flex flex-flow gap-2',
						'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
					)}
					role="menuitem"
					tabindex="-1"
					use:melt={item}
				>
					<Settings size={16} />
					Workspace settings
				</a>
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
					<div
						class="bg-blue-400 h-1"
						style="width: {Math.min($workspaceUsageStore, 1000) / 10}%"
					/>
				</div>
			{/if}
			{#if $userStore?.is_admin}
				<button
					type="button"
					class={twMerge(
						'text-secondary block font-normal w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900',
						'data-[highlighted]:bg-gray-100 data-[highlighted]:text-gray-900'
					)}
					role="menuitem"
					tabindex="-1"
					on:click={() => {
						goto('/workspace_settings?tab=premium')
					}}
					use:melt={item}
				>
					Upgrade
				</button>
			{/if}
		</div>
	{/if}
	{#if $enterpriseLicense && !strictWorkspaceSelect}
		<MultiplayerMenu />
	{/if}
</Menu>
