<script lang="ts">
	import { goto } from '$app/navigation'
	import { logout } from '$lib/logout'
	import { userStore, usersWorkspaceStore, usageStore, premiumStore } from '$lib/stores'
	import Menu from '../common/menu/MenuV2.svelte'
	import { USER_SETTINGS_HASH } from './settings'
	import { isCloudHosted } from '$lib/cloud'
	import { twMerge } from 'tailwind-merge'
	import { Crown, HardHat, LogOut, Moon, Settings, Sun, User } from 'lucide-svelte'
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
		<MenuButton
			class="!text-xs"
			icon={User}
			label={`User (${$userStore?.username ?? $userStore?.email})`}
			{isCollapsed}
		/>
	</div>
	<div class="divide-y">
		<div class="px-4 py-3" role="none">
			<p class="text-sm font-medium text-primary truncate" role="none">
				{$usersWorkspaceStore?.email}
			</p>
			<span class="text-xs text-tertiary flex flex-row gap-2 items-center">
				{#if $userStore?.is_admin}
					Admin of this workspace <Crown size={14} />
				{:else if $userStore?.operator}
					Operator in this workspace <HardHat size={14} />
				{/if}
			</span>
		</div>

		<div class="py-1" role="none">
			<MenuItem
				href={USER_SETTINGS_HASH}
				class={twMerge(
					'flex flex-row gap-2 items-center px-4 py-2 ',
					'text-secondary text-sm',
					'hover:bg-surface-hover hover:text-primary cursor-pointer'
				)}
			>
				<Settings size={14} />
				Account settings
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
					'flex flex-row items-center gap-2'
				)}
				role="menuitem"
				tabindex="-1"
			>
				{#if darkMode}
					<Sun size={14} />
				{:else}
					<Moon size={14} />
				{/if}
				Switch theme
			</button>
		</div>

		<div class="py-1" role="none">
			<MenuItem
				on:click={() => logout()}
				class={twMerge(
					'flex flex-row gap-2 items-center px-4 py-2 ',
					'text-secondary text-sm',
					'hover:bg-surface-hover hover:text-primary cursor-pointer'
				)}
			>
				<LogOut size={14} />
				Sign out
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
