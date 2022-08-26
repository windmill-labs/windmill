<script>
	import { workspaceStore, usersWorkspaceStore } from '$lib/stores'
	import { faCog } from '@fortawesome/free-solid-svg-icons'

	import { onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { scale } from 'svelte/transition'

	let show = false
	let menu = null

	const handleOutsideClick = (event) => {
		if (show && !menu.contains(event.target)) {
			show = false
		}
	}

	const handleEscape = (event) => {
		if (show && event.key === 'Escape') {
			show = false
		}
	}

	onMount(() => {
		document.addEventListener('click', handleOutsideClick, false)
		document.addEventListener('keyup', handleEscape, false)

		return () => {
			document.removeEventListener('click', handleOutsideClick, false)
			document.removeEventListener('keyup', handleEscape, false)
		}
	})
</script>

<div class="relative" bind:this={menu}>
	<div>
		<button
			type="button"
			on:click={() => (show = !show)}
			class="inline-flex items-center text-white bg-gray-700 hover:bg-gray-800 focus:ring-4 focus:outline-none focus:ring-gray-300 font-medium rounded-lg text-xs px-3 py-2 text-center"
		>
			<Icon data={faCog} scale={0.8} class="mr-2" />
			Workspace
		</button>

		{#if show}
			<div
				in:scale={{ duration: 100, start: 0.95 }}
				out:scale={{ duration: 75, start: 0.95 }}
				class="z-50 origin-top-right absolute right-0 mt-2 w-56 rounded-md shadow-lg bg-white ring-1 ring-black ring-opacity-5 divide-y divide-gray-100 focus:outline-none"
				role="menu"
				aria-orientation="vertical"
				aria-labelledby="workspace-menu"
				tabindex="-1"
			>
				<div class="py-1" role="none">
					{#each $usersWorkspaceStore?.workspaces ?? [] as workspace}
						<button
							on:click={() => {
								workspaceStore.set(workspace.id)
								show = false
							}}
							class="block px-4 py-2 text-xs text-gray-500 "
							role="menuitem"
							tabindex="-1"
							id="user-menu-item-2"
						>
							<span class="text-gray-300 font-mono pr-1 text-xs">{workspace.id}</span
							>{workspace.name}
						</button>
					{/each}
					<a
						href="/user/create_workspace"
						class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
						role="menuitem"
						tabindex="-1"
					>
						Create new workspace
					</a>
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
		{/if}
	</div>
</div>
