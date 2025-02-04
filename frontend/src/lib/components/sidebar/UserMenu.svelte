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
	import MenuButtonMelt from './MenuButtonMelt.svelte'

	import { melt } from '@melt-ui/svelte'
	import Menu from '$lib/components/meltComponents/Menu.svelte'
	let darkMode: boolean = false
	export let isCollapsed: boolean = false
	export let lightMode: boolean = false
	export let createMenu: (any) => any
</script>

<Menu {createMenu} let:item>
	<svelte:fragment slot="trigger" let:trigger>
		<MenuButtonMelt
			class="!text-xs"
			icon={User}
			label={`User (${$userStore?.username ?? $userStore?.email})`}
			{isCollapsed}
			{lightMode}
			{trigger}
		/>
	</svelte:fragment>
	<div class="divide-y z-20">
		<div class="px-4 py-3" role="none">
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

		<div class="py-1" role="none">
			<a
				href={USER_SETTINGS_HASH}
				class={twMerge(
					'flex flex-row gap-2 items-center px-4 py-2 ',
					'text-secondary text-sm',
					'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
				)}
				use:melt={item}
			>
				<Settings size={14} />
				Account settings
			</a>
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
					'text-secondary block text-left px-4 py-2 font-normal text-sm w-full',
					'flex flex-row items-center gap-2',
					'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
				)}
				role="menuitem"
				tabindex="-1"
				use:melt={item}
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
			<button
				on:click={() => logout()}
				class={twMerge(
					'flex flex-row gap-2 items-center px-4 py-2 w-full',
					'text-secondary text-sm',
					'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
				)}
				use:melt={item}
			>
				<LogOut size={14} />
				Sign out
			</button>
		</div>

		{#if isCloudHosted()}
			{#if !$isPremiumStore}
				<div class="py-1" role="none">
					<span class="text-secondary block w-full text-left px-4 py-2 text-sm"
						>{$usageStore}/1000 user execs</span
					>
					<div class="w-full bg-gray-200 h-1">
						<div class="bg-blue-400 h-1" style="width: {Math.min($usageStore, 1000) / 10}%" />
					</div>
					{#if $workspaceStore != 'demo'}
						<span class="text-secondary block w-full text-left px-4 py-2 text-xs"
							>{$workspaceUsageStore}/1000 free workspace execs</span
						>
						<div class="w-full bg-gray-200 h-1">
							<div
								class="bg-blue-400 h-1"
								style="width: {Math.min($workspaceUsageStore, 1000) / 10}%"
							/>
						</div>
					{/if}
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

<DarkModeObserver bind:darkMode />
