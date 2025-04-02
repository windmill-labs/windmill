<script lang="ts">
	import { goto } from '$lib/navigation'
	import { logout } from '$lib/logout'
	import {
		userStore,
		usageStore,
		workspaceUsageStore,
		isPremiumStore,
		workspaceStore
	} from '$lib/stores'
	import { USER_SETTINGS_HASH } from './settings'
	import { isCloudHosted } from '$lib/cloud'
	import { twMerge } from 'tailwind-merge'
	import { Crown, HardHat, LogOut, Moon, Settings, Sun, User } from 'lucide-svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import MenuButton from './MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import { type MenubarBuilders } from '@melt-ui/svelte'

	let darkMode: boolean = false

	export let isCollapsed: boolean = false
	export let lightMode: boolean = false
	export let createMenu: MenubarBuilders['createMenu']

	const itemClass = twMerge(
		'text-secondary text-left font-normal text-xs ',
		'flex flex-row items-center gap-2 px-4 py-3 w-full',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
</script>

<Menu {createMenu} let:item usePointerDownOutside>
	<svelte:fragment slot="trigger" let:trigger>
		<MenuButton
			class="!text-xs"
			icon={User}
			label={`User (${$userStore?.username ?? $userStore?.email})`}
			{isCollapsed}
			{lightMode}
			{trigger}
		/>
	</svelte:fragment>

	<div class="px-4 py-3 border-b" role="none">
		<p class="text-sm font-medium text-primary truncate" role="none">
			{$userStore?.email}
		</p>
		<span class="text-xs text-tertiary flex flex-row gap-2 items-center">
			{#if $userStore?.is_admin}
				Admin of this workspace <Crown size={14} />
			{:else if $userStore?.operator}
				Operator in this workspace <HardHat size={14} />
			{/if}
		</span>
	</div>
	<div class="py-1">
		<MenuItem href={USER_SETTINGS_HASH} class={itemClass} {item}>
			<Settings size={16} />
			Account settings
		</MenuItem>

		<MenuItem
			on:click={() => {
				if (!document.documentElement.classList.contains('dark')) {
					document.documentElement.classList.add('dark')
					window.localStorage.setItem('dark-mode', 'dark')
				} else {
					document.documentElement.classList.remove('dark')
					window.localStorage.setItem('dark-mode', 'light')
				}
			}}
			class={itemClass}
			{item}
		>
			{#if darkMode}
				<Sun size={16} />
			{:else}
				<Moon size={16} />
			{/if}
			Switch theme
		</MenuItem>

		<MenuItem
			on:click={() => logout()}
			class={twMerge(itemClass, 'text-primary font-semibold')}
			{item}
		>
			<LogOut size={16} />
			Sign out
		</MenuItem>
	</div>

	{#if isCloudHosted()}
		<div class="border-t">
			{#if !$isPremiumStore}
				<span class="text-secondary block w-full text-left px-4 py-2 text-xs"
					>{$usageStore}/1000 user execs</span
				>
				<div class="px-4 w-full h-1 mb-1">
					<div class="bg-gray-200 h-full rounded-sm overflow-hidden">
						<div class="bg-blue-400 h-full" style="width: {Math.min($usageStore, 1000) / 10}%"></div>
					</div>
				</div>
				{#if $workspaceStore != 'demo'}
					<span class="text-secondary block w-full text-left px-4 py-2 text-xs"
						>{$workspaceUsageStore}/1000 free workspace execs</span
					>
					<div class="px-4 w-full h-1 mb-1">
						<div class="bg-gray-200 h-full rounded-sm overflow-hidden">
							<div
								class="bg-blue-400 h-full"
								style="width: {Math.min($workspaceUsageStore, 1000) / 10}%"
							></div>
						</div>
					</div>
				{/if}
				{#if $userStore?.is_admin}
					<MenuItem
						class={twMerge(itemClass, 'py-2')}
						on:click={() => {
							goto('/workspace_settings?tab=premium')
						}}
						{item}
					>
						Upgrade
					</MenuItem>
				{/if}
			{:else}
				<MenuItem
					class={twMerge(itemClass, 'py-2')}
					on:click={() => {
						goto('/workspace_settings?tab=premium')
					}}
					{item}
				>
					Premium plan
				</MenuItem>
			{/if}
		</div>
	{/if}
</Menu>

<DarkModeObserver bind:darkMode />
