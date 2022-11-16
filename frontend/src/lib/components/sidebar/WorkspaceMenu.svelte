<script lang="ts">
	import { workspaceStore, usersWorkspaceStore } from '$lib/stores'
	import { classNames } from '$lib/utils'
	import { faBuilding } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	import Menu from '../common/menu/Menu.svelte'

	export let isCollapsed: boolean = false
</script>

<Menu placement="bottom-start" let:close>
	<button
		slot="trigger"
		type="button"
		class={classNames(
			'group w-full flex items-center text-white hover:bg-gray-50 hover:text-gray-900 focus:ring-4 focus:outline-none focus:ring-gray-300 px-2 py-2 text-sm font-medium rounded-md h-8 '
		)}
	>
		<Icon
			data={faBuilding}
			class={classNames('flex-shrink-0 h-4 w-4', isCollapsed ? '-mr-1' : 'mr-2')}
		/>

		{#if !isCollapsed}
			<span class={classNames('whitespace-pre truncate')}> {$workspaceStore} </span>
		{/if}
	</button>

	<div class="divide-y divide-gray-100" role="none">
		<table class="w-full">
			{#each $usersWorkspaceStore?.workspaces ?? [] as workspace}
				<tr
					class="text-xs cursor-pointer"
					on:click={() => {
						workspaceStore.set(workspace.id)
						close()
					}}
				>
					<td class="text-gray-500 font-mono pl-4 pr-1 py-2 text-xs whitespace-nowrap"
						>{workspace.id}</td
					>
					<td class="text-gray-700 pr-4 py-2 w-full">{workspace.name}</td>
				</tr>
			{/each}
		</table>
		<div class="py-1" role="none">
			<a
				href="/user/create_workspace"
				class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
				role="menuitem"
				tabindex="-1"
			>
				Create new workspace
			</a>
		</div>
		<div class="py-1" role="none">
			<a
				href="/user/workspaces"
				on:click={() => {
					localStorage.removeItem('workspace')
				}}
				class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
				role="menuitem"
				tabindex="-1"
			>
				See all workspaces & invites
			</a>
		</div>
	</div>
</Menu>
