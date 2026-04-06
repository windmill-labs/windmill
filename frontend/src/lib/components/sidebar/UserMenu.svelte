<script lang="ts">
	import { goto } from '$lib/navigation'
	import { logout } from '$lib/logoutKit'
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
	import { Crown, ServerCog, LogOut, Moon, Settings, Sun, User, Languages } from 'lucide-svelte'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import MenuButton from './MenuButton.svelte'
	import { Menu, MenuItem } from '$lib/components/meltComponents'
	import { type MenubarBuilders } from '@melt-ui/svelte'
	import { t, getLocale, getLocaleChoice, setLocale } from '$lib/i18n/t.svelte'
	import { SUPPORTED_LOCALES, LOCALE_NAMES } from '$lib/i18n/index'

	let showLanguages = $state(false)

	let darkMode: boolean = $state(false)

	interface Props {
		isCollapsed?: boolean
		lightMode?: boolean
		createMenu: MenubarBuilders['createMenu']
	}

	let { isCollapsed = false, lightMode = false, createMenu }: Props = $props()

	const itemClass = twMerge(
		'text-secondary text-left font-normal text-xs ',
		'flex flex-row items-center gap-2 px-4 py-3 w-full',
		'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
</script>

<Menu {createMenu} usePointerDownOutside>
	{#snippet triggr({ trigger })}
		<MenuButton
			class="!text-xs"
			icon={User}
			label={`User (${$userStore?.username ?? $userStore?.email})`}
			{isCollapsed}
			{lightMode}
			{trigger}
		/>
	{/snippet}

	{#snippet children({ item })}
		<div class="px-4 py-3 border-b" role="none">
			<p class="text-sm font-medium text-primary truncate" role="none">
				{$userStore?.email}
			</p>
			<span class="text-xs text-primary flex flex-row gap-2 items-center">
				{#if $userStore?.is_admin}
					{t('user.admin_of_workspace')} <Crown size={14} />
				{:else if $userStore?.operator}
					{t('user.operator_in_workspace')} <ServerCog size={14} />
				{/if}
			</span>
		</div>
		<div class="py-1">
			<MenuItem href={USER_SETTINGS_HASH} class={itemClass} {item}>
				<Settings size={16} />
				{t('user.account_settings')}
			</MenuItem>

			<MenuItem
				onClick={() => {
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
				{t('user.switch_theme')}
			</MenuItem>

			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class={twMerge(itemClass, 'cursor-pointer')}
				role="button"
				tabindex="0"
				onclick={(e) => {
					e.stopPropagation()
					showLanguages = !showLanguages
				}}
			>
				<Languages size={16} />
				{t('user.language')}: {getLocaleChoice() === 'auto'
					? 'Detected'
					: LOCALE_NAMES[getLocale()]}
			</div>
			{#if showLanguages}
				<div class="border-t border-b py-1">
					<MenuItem
						onClick={() => setLocale('auto')}
						class={twMerge(
							itemClass,
							'pl-8',
							getLocaleChoice() === 'auto' ? 'font-semibold text-primary' : ''
						)}
						{item}
					>
						Detected ({LOCALE_NAMES[getLocale()]})
					</MenuItem>
					{#each SUPPORTED_LOCALES as loc (loc)}
						<MenuItem
							onClick={() => setLocale(loc)}
							class={twMerge(
								itemClass,
								'pl-8',
								getLocaleChoice() === loc ? 'font-semibold text-primary' : ''
							)}
							{item}
						>
							{LOCALE_NAMES[loc]}
						</MenuItem>
					{/each}
				</div>
			{/if}

			<MenuItem onClick={() => logout()} class={itemClass} {item}>
				<LogOut size={16} />
				{t('user.sign_out')}
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
							<div class="bg-blue-400 h-full" style="width: {Math.min($usageStore, 1000) / 10}%"
							></div>
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
							onClick={() => {
								goto('/workspace_settings?tab=premium')
							}}
							{item}
						>
							{t('user.upgrade')}
						</MenuItem>
					{/if}
				{:else}
					<MenuItem
						class={twMerge(itemClass, 'py-2')}
						onClick={() => {
							goto('/workspace_settings?tab=premium')
						}}
						{item}
					>
						{t('user.premium_plan')}
					</MenuItem>
				{/if}
			</div>
		{/if}
	{/snippet}
</Menu>

<DarkModeObserver bind:darkMode />
