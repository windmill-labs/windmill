<script lang="ts">
	import { goto } from '$app/navigation'
	import { logout } from '$lib/logout'
	import { userStore, usersWorkspaceStore, usageStore, premiumStore } from '$lib/stores'
	import { faCog, faCrown, faHardHat, faSignOut } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Menu from '../common/menu/MenuV2.svelte'
	import { USER_SETTINGS_HASH } from './settings'
	import { isCloudHosted } from '$lib/cloud'
	import { twMerge } from 'tailwind-merge'
	import { Moon, Sun, User } from 'lucide-svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

	let darkMode: boolean = document.documentElement.classList.contains('dark')
	export let isCollapsed: boolean = false
	function onThemeChange() {
		if (document.documentElement.classList.contains('dark')) {
			darkMode = true
		} else {
			darkMode = false
		}
	}
</script>

<Menu>
	<div slot="trigger" class="w-full">
		<MenuButton class="!text-xs" icon={User} label={'User'} {isCollapsed} />
	</div>
	<div class="divide-y">
		<div class="px-4 py-3" role="none">
			<p class="text-sm font-medium text-primary truncate" role="none">
				{$usersWorkspaceStore?.email}
			</p>
			<span class="text-xs text-tertiary">
				{#if $userStore?.is_admin}
					Admin of this workspace <Icon data={faCrown} scale={0.6} />
				{:else if $userStore?.operator}
					Operator in this workspace <Icon class="ml-2" data={faHardHat} scale={0.8} />
				{/if}
			</span>
		</div>

		<div class="py-1" role="none">
			<MenuItem>
				<a
					href={USER_SETTINGS_HASH}
					class="text-secondary block px-4 py-2 text-sm hover:bg-surface-hover hover:text-primary"
					role="menuitem"
					tabindex="-1"
				>
					<Icon class="pr-0.5" data={faCog} /> Account settings
				</a>
			</MenuItem>
		</div>

		<div class="py-1" role="none">
			<button
				on:click={() => {
					if (!document.documentElement.classList.contains('dark')) {
						document.documentElement.classList.add('dark')
						window.localStorage.setItem('dark-mode', 'dark')
					} else {
						document.documentElement.classList.remove('dark')
						window.localStorage.setItem('dark-mode', 'light')
					}
				}}
				class={twMerge(
					'text-secondary block text-left px-4 py-2 font-normal text-sm hover:bg-surface-hover hover:text-primary w-full',
					'flex flex-row items-center gap-1'
				)}
				role="menuitem"
				tabindex="-1"
			>
				{#if darkMode}
					<Sun class="w-4 h-4" />
				{:else}
					<Moon class="w-4 h-4" />
				{/if}
				Switch theme
			</button>
		</div>

		<div class="py-1" role="none">
			<MenuItem>
				<button
					type="button"
					class="text-secondary block w-full text-left px-4 py-2 text-sm hover:bg-surface-hover hover:text-primary"
					role="menuitem"
					tabindex="-1"
					on:click={() => logout()}
				>
					<Icon class="pr-0.5" data={faSignOut} /> Sign out
				</button>
			</MenuItem>
		</div>

		{#if isCloudHosted() && $premiumStore}
			{#if !$premiumStore.premium}
				<div class="py-1" role="none">
					<span class="text-secondary block w-full text-left px-4 py-2 text-sm"
						>{$usageStore}/1000 free-tier executions</span
					>
					<div class="w-full bg-gray-200 h-1">
						<div class="bg-blue-400 h-1" style="width: {Math.min($usageStore, 1000) / 10}%" />
					</div>
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
			{:else}
				<div class="py-1" role="none">
					<button
						type="button"
						class="text-secondary block font-normal w-full text-left px-4 py-2 text-sm hover:bg-gray-100 hover:text-gray-900"
						role="menuitem"
						tabindex="-1"
						on:click={() => {
							goto('/workspace_settings?tab=premium')
						}}
					>
						Premium plan
					</button>
				</div>
			{/if}
		{/if}
	</div>
</Menu>

<DarkModeObserver on:change={onThemeChange} />
