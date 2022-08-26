<script>
	import { logout } from '$lib/logout'

	import { userStore, usersWorkspaceStore, superadmin } from '$lib/stores'
	import { faCrown } from '@fortawesome/free-solid-svg-icons'

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
		<button on:click={() => (show = !show)} class="menu focus:outline-none focus:shadow-solid">
			<div class="flex items-center">
				<span class="inline-block h-8 w-8 rounded-full overflow-hidden bg-gray-100">
					<svg class="h-full w-full text-gray-300" fill="currentColor" viewBox="0 0 24 24">
						<path
							d="M24 20.993V24H0v-2.996A14.977 14.977 0 0112.004 15c4.904 0 9.26 2.354 11.996 5.993zM16.002 8.999a4 4 0 11-8 0 4 4 0 018 0z"
						/>
					</svg>
				</span>
				<div class="ml-3">
					<p class="text-sm font-medium text-gray-700 group-hover:text-gray-900">
						{$userStore?.username ?? ($superadmin ? $superadmin : '___')}
						{#if $userStore?.is_admin}
							<Icon data={faCrown} scale={0.6} />
						{/if}
					</p>
					<p class="text-xs font-medium text-gray-500 group-hover:text-gray-700">View profile</p>
				</div>
			</div>
		</button>

		{#if show}
			<div
				in:scale={{ duration: 100, start: 0.95 }}
				out:scale={{ duration: 75, start: 0.95 }}
				class="z-50 origin-top-right absolute right-0 mt-2 w-56 rounded-md shadow-lg bg-white ring-1 ring-black ring-opacity-5 divide-y divide-gray-100 focus:outline-none"
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
