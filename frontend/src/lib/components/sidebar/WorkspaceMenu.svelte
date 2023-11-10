<script lang="ts">
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { Building, Plus, Settings } from 'lucide-svelte'

	import Menu from '../common/menu/MenuV2.svelte'
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

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
				href="/user/create_workspace"
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
				href="/user/workspaces"
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
		<div class="py-1" role="none">
			<MenuItem>
				<a
					href="/workspace_settings"
					class="text-secondary px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary flex flex-flow gap-2"
					role="menuitem"
					tabindex="-1"
				>
					<Settings size={16} />
					Workspace Settings
				</a>
			</MenuItem>
		</div>
	</div>
	{#if $enterpriseLicense}
		<MultiplayerMenu />
	{/if}
</Menu>
