<script lang="ts">
	import {
		Home,
		Menu as MenuIcon,
		Play,
		Settings,
		LogOut,
		Moon,
		Sun,
		Code2,
		LayoutDashboard,
		Building
	} from 'lucide-svelte'

	import Menu from '../common/menu/MenuV2.svelte'

	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense } from '$lib/stores'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import MenuLink from './MenuLink.svelte'
	import { twMerge } from 'tailwind-merge'
	import { USER_SETTINGS_HASH } from './settings'
	import { logout } from '$lib/logout'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import BarsStaggered from '../icons/BarsStaggered.svelte'

	let darkMode: boolean = false

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app' | 'raw_app'
	}[]

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: Home },
		{ label: 'Runs', href: '/runs', icon: Play }
	]
</script>

<Menu>
	<div slot="trigger">
		<MenuButton class="!text-xs" icon={MenuIcon} {isCollapsed} lightMode label={undefined} />
	</div>
	<div class="w-full max-w-full">
		{#each favoriteLinks ?? [] as favorite (favorite.href)}
			<MenuItem href={favorite.href}>
				<span class="w-full inline-flex flex-row px-2 py-2 hover:bg-surface-hover">
					<span class="center-center">
						{#if favorite.kind == 'script'}
							<Code2 size={16} />
						{:else if favorite.kind == 'flow'}
							<BarsStaggered size={16} />
						{:else if favorite.kind == 'app' || favorite.kind == 'raw_app'}
							<LayoutDashboard size={16} />
						{/if}
					</span>
					<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
						{favorite.label}
					</span>
				</span>
			</MenuItem>
		{/each}
	</div>

	{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
		<MenuLink class="!text-xs" {...menuLink} {isCollapsed} lightMode />
	{/each}

	<div class="divide-y" role="none">
		<div role="none">
			<MenuItem
				href={USER_SETTINGS_HASH}
				class={twMerge(
					'flex flex-row gap-3.5 items-center px-2 py-2 ',
					'text-secondary text-xs',
					'hover:bg-surface-hover hover:text-primary cursor-pointer'
				)}
			>
				<Settings size={14} />
				Account settings
			</MenuItem>
		</div>

		<div role="none">
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
					'text-secondary block text-left px-2 py-2 font-normal text-xs hover:bg-surface-hover hover:text-primary w-full',
					'flex flex-row items-center gap-3.5 '
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
			<a
				href="/user/workspaces"
				on:click={() => {
					localStorage.removeItem('workspace')
				}}
				class="text-primary flex gap-3.5 px-2 py-2 text-xs hover:bg-surface-hover hover:text-primary"
				role="menuitem"
				tabindex="-1"
			>
				<Building size={14} />
				All Workspaces
			</a>

			<MenuItem
				href="#"
				on:click={() => logout()}
				class={twMerge(
					'flex flex-row gap-3.5  items-center px-2 py-2 ',
					'text-secondary text-xs',
					'hover:bg-surface-hover hover:text-primary cursor-pointer'
				)}
			>
				<LogOut size={14} />
				Sign out
			</MenuItem>
		</div>
	</div>
	{#if $enterpriseLicense}
		<MultiplayerMenu />
	{/if}
</Menu>

<DarkModeObserver bind:darkMode />
