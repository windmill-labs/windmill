<script lang="ts">
	import { logout } from '$lib/logout'

	import { userStore, usersWorkspaceStore, superadmin } from '$lib/stores'
	import { classNames } from '$lib/utils'
	import { faCrown, faUser } from '@fortawesome/free-solid-svg-icons'

	import { onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import { scale } from 'svelte/transition'

	let show = false
	let menu: HTMLDivElement

	export let isCollapsed: boolean = false

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
			class={classNames(
				'group w-full flex items-center text-white hover:bg-gray-50 hover:text-gray-900  focus:outline-none  px-2 py-2 text-sm font-medium rounded-md h-8 '
			)}
			on:click={() => (show = !show)}
		>
			<Icon
				data={faUser}
				class={classNames('flex-shrink-0 h-4 w-4', isCollapsed ? '-mr-1' : 'mr-2')}
			/>

			{#if !isCollapsed}
				<span class={classNames('whitespace-pre ')}>
					{$userStore?.username ?? ($superadmin ? $superadmin : '___')}
					{#if $userStore?.is_admin}
						<Icon data={faCrown} scale={0.6} />
					{/if}
				</span>
			{/if}
		</button>

		{#if show}
			<div
				in:scale={{ duration: 100, start: 0.95 }}
				out:scale={{ duration: 75, start: 0.95 }}
				class="z-50 origin-top-left absolute  left-0 mt-2 w-56 rounded-md shadow-lg bg-white ring-1 ring-black ring-opacity-5 divide-y divide-gray-100 focus:outline-none"
				role="menu"
				tabindex="-1"
			>
				<div class="px-4 py-3" role="none">
					<p class="text-sm" role="none">Signed in as</p>
					<p class="text-sm font-medium text-gray-900 truncate" role="none">
						{$usersWorkspaceStore?.email}
					</p>
				</div>
				<div class="py-1" role="none">
					<a
						href="/user/settings"
						class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
						role="menuitem"
						tabindex="-1"
					>
						Account settings
					</a>
				</div>
				<div class="py-1" role="none">
					<button
						type="button"
						class="text-gray-700 block w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
						role="menuitem"
						tabindex="-1"
						on:click={() => logout()}
					>
						Sign out
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
