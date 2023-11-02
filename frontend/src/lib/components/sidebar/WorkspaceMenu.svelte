<script lang="ts">
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { Building } from 'lucide-svelte'

	import Menu from '../common/menu/MenuV2.svelte'
	import { faCog, faPlus } from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'
	import { goto } from '$app/navigation'
	import { dirtyStore } from '../common/confirmationModal/dirtyStore'
	import { page } from '$app/stores'
	import { switchWorkspace } from '$lib/storeUtils'
	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

	export let isCollapsed: boolean = false

	function waitForNextUpdate(store) {
		return new Promise((resolve) => {
			let firstEmission = true
			store.subscribe((value) => {
				if (firstEmission) {
					firstEmission = false
					return
				}
				resolve(value)
			})
		})
	}

	async function toggleSwitchWorkspace(id: string) {
		if ($workspaceStore === id) {
			return
		}

		const editPages = ['/scripts/edit/', '/flows/edit/', '/apps/edit/']
		const isOnEditPage = editPages.some((editPage) => $page.route.id?.includes(editPage) ?? false)

		// Check if we have unsaved changes
		const wasDirty = $dirtyStore
		// Try to go to the home page

		// If we weren't dirty, we can directly switch workspaces
		if (!wasDirty && !isOnEditPage) {
			switchWorkspace(id)
		} else {
			await goto('/')

			const isStillDirty = await waitForNextUpdate(dirtyStore)

			if (!isStillDirty) {
				switchWorkspace(id)
			}
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
							close()
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
				class="text-primary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary"
				role="menuitem"
				tabindex="-1"
			>
				<Icon data={faPlus} class="-mt-0.5 pr-0.5" /> Workspace
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
					class="text-secondary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary"
					role="menuitem"
					tabindex="-1"
				>
					<Icon class="pr-0.5" data={faCog} /> Workspace Settings
				</a>
			</MenuItem>
		</div>
	</div>
	{#if $enterpriseLicense}
		<MultiplayerMenu />
	{/if}
</Menu>
