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

	import Menu from '../common/menu/MenuV2.svelte'
	import { goto } from '$lib/navigation'
	import { base } from '$lib/navigation'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { isCloudHosted } from '$lib/cloud'

	export let isCollapsed: boolean = false

	async function toggleSwitchWorkspace(id: string) {
		if ($workspaceStore === id) {
			return
		}

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
		} else {
			await goto('/')
			switchWorkspace(id)
		}
	}
</script>

<Menu>
	<div slot="trigger">
		<MenuButton class="!text-xs" icon={Building} label={$workspaceStore ?? ''} {isCollapsed} />
	</div>

	<div class="divide-y" role="none">
		<div class="py-1">
			{#each $userWorkspaces as workspace}
				<MenuItem>
					<button
						class="text-xs min-w-0 w-full overflow-hidden flex flex-col py-1.5
						{$workspaceStore === workspace.id
							? 'cursor-default bg-surface-selected'
							: 'cursor-pointer hover:bg-surface-hover'}"
						on:click={async () => {
							await toggleSwitchWorkspace(workspace.id)
						}}
					>
						<div class="text-primary pl-4 truncate text-left text-[1.2em]">{workspace.name}</div>
						<div class="text-tertiary font-mono pl-4 text-2xs whitespace-nowrap truncate text-left">
							{workspace.id}
						</div>
					</button>
				</MenuItem>
			{/each}
		</div>
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
		<div class="py-1" role="none">
			<a
				href="{base}/user/workspaces"
				on:click={() => {
					localStorage.removeItem('workspace')
				}}
				class="text-primary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary"
				role="menuitem"
				tabindex="-1"
			>
				All workspaces & invites
			</a>
		</div>
		{#if $userStore?.is_admin || $superadmin}
			<div class="py-1" role="none">
				<MenuItem>
					<a
						href="{base}/workspace_settings"
						class="text-secondary px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary flex flex-flow gap-2"
						role="menuitem"
						tabindex="-1"
					>
						<Settings size={16} />
						Workspace Settings
					</a>
				</MenuItem>
			</div>
		{/if}
	</div>
	{#if isCloudHosted() && !$isPremiumStore}
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
					class="text-secondary block font-normal w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
					role="menuitem"
					tabindex="-1"
					on:click={() => {
						goto('/workspace_settings?tab=premium')
					}}
				>
					Upgrade
				</button>
			{/if}
		</div>
	{/if}
	{#if $enterpriseLicense}
		<MultiplayerMenu />
	{/if}
</Menu>
