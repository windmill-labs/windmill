<script lang="ts">
	import { logout } from '$lib/logout'

	import { userStore, usersWorkspaceStore, superadmin } from '$lib/stores'
	import { classNames } from '$lib/utils'
	import { faCrown, faUser } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'

	import Icon from 'svelte-awesome'
	import Menu from '../common/menu/Menu.svelte'

	let dispatch = createEventDispatcher()
	export let isCollapsed: boolean = false
</script>

<Menu placement="bottom-start">
	<button
		slot="trigger"
		type="button"
		class={classNames(
			'group w-full flex items-center text-white hover:bg-gray-50 hover:text-gray-900  focus:outline-none  px-2 py-2 text-sm font-medium rounded-md h-8 '
		)}
	>
		<Icon
			data={faUser}
			class={classNames('flex-shrink-0 h-4 w-4', isCollapsed ? '-mr-1' : 'mr-2')}
		/>

		{#if !isCollapsed}
			<span class={classNames('whitespace-pre truncate')}>
				{$userStore?.username ?? ($superadmin ? $superadmin : '___')}
				{#if $userStore?.is_admin || $superadmin}
					<Icon data={faCrown} scale={0.6} />
				{/if}
			</span>
		{/if}
	</button>

	<div class="divide-y divide-gray-100">
		<div class="px-4 py-3" role="none">
			<p class="text-sm" role="none">Signed in as</p>
			<p class="text-sm font-medium text-gray-900 truncate" role="none">
				{$usersWorkspaceStore?.email}
			</p>
		</div>
		<div class="py-1" role="none">
			<a
				on:click={() => dispatch('user-settings')}
				href="#user-settings"
				class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
				role="menuitem"
				tabindex="-1"
			>
				Account settings
			</a>
		</div>
		{#if $superadmin}
			<div class="py-1" role="none">
				<a
					on:click={() => dispatch('superadmin-settings')}
					href="#superadmin-settings"
					class="text-gray-700 block px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
					role="menuitem"
					tabindex="-1"
				>
					Superadmin settings
				</a>
			</div>
		{/if}
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
</Menu>
