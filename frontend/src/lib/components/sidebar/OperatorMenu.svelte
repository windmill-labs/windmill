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
		Building,
		Calendar,
		ServerCog
	} from 'lucide-svelte'
	import { base } from '$lib/base'

	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { enterpriseLicense, superadmin, userWorkspaces, workspaceStore } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { USER_SETTINGS_HASH } from './settings'
	import { logout } from '$lib/logout'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import BarsStaggered from '../icons/BarsStaggered.svelte'
	import Menu from '$lib/components/meltComponents/Menu.svelte'
	import Menubar from '$lib/components/meltComponents/Menubar.svelte'
	import MenuButtonMelt from './MenuButtonMelt.svelte'
	import MenuLinkMelt from './MenuLinkMelt.svelte'
	import { melt } from '@melt-ui/svelte'
	let darkMode: boolean = false

	export let isCollapsed: boolean = false
	export let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'script' | 'flow' | 'app' | 'raw_app'
	}[]

	$: mainMenuLinks = [
		{ label: 'Home', id: 'home', href: `${base}/`, icon: Home },
		{ label: 'Runs', id: 'runs', href: `${base}/runs`, icon: Play },
		{ label: 'Schedules', id: 'schedules', href: `${base}/schedules`, icon: Calendar }
	].filter(
		(link) =>
			link.id === 'home' ||
			($userWorkspaces &&
				$workspaceStore &&
				$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.[link.id] ===
					true)
	)

	$: secondMenuLinks = [
		{
			label: 'Resources',
			id: 'resources',
			href: `${base}/resources`
		},
		{
			label: 'Variables',
			id: 'variables',
			href: `${base}/variables`
		},
		{
			label: 'Custom HTTP routes',
			id: 'triggers',
			href: `${base}/routes`
		},
		{
			label: 'Websocket triggers',
			id: 'triggers',
			href: `${base}/websocket_triggers`
		},
		{
			label: 'Postgres triggers',
			id: 'triggers',
			href: `${base}/postgres_triggers`
		},
		{
			label: 'Kafka triggers',
			id: 'triggers',
			href: `${base}/kafka_triggers`
		},
		{
			label: 'NATS triggers',
			id: 'triggers',
			href: `${base}/nats_triggers`
		},
		{
			label: 'Audit logs',
			id: 'audit_logs',
			href: `${base}/audit_logs`
		},
		{
			label: 'Groups',
			id: 'groups',
			href: `${base}/groups`
		},
		{
			label: 'Folders',
			id: 'folders',
			href: `${base}/folders`
		},
		{
			label: 'Workers',
			id: 'workers',
			href: `${base}/workers`
		}
	].filter((link) => {
		if (!$userWorkspaces || !$workspaceStore) return false
		return (
			$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.[link.id] === true
		)
	})

	let moreOpen = false
</script>

<Menubar let:createMenu>
	<Menu {createMenu} let:item>
		<svelte:fragment slot="trigger" let:trigger>
			<MenuButtonMelt
				class="!text-xs"
				icon={MenuIcon}
				{isCollapsed}
				lightMode
				label={undefined}
				{trigger}
			/>
		</svelte:fragment>
		<div class="w-full max-w-full">
			{#each favoriteLinks ?? [] as favorite (favorite.href)}
				<a
					href={favorite.href}
					use:melt={item}
					class={twMerge(
						'w-full inline-flex flex-row px-2 py-2 hover:bg-surface-hover',
						'data-[highlighted]:bg-surface-hover'
					)}
				>
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
				</a>
			{/each}
		</div>

		{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
			<MenuLinkMelt class="!text-xs" {...menuLink} {isCollapsed} lightMode {item} />
		{/each}

		<div class="divide-y" role="none">
			<div role="none">
				<a
					href={USER_SETTINGS_HASH}
					class={twMerge(
						'flex flex-row gap-3.5 items-center px-2 py-2 ',
						'text-secondary text-xs',
						'hover:bg-surface-hover hover:text-primary cursor-pointer',
						'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
					)}
					use:melt={item}
				>
					<Settings size={14} />
					Account settings
				</a>
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
						'flex flex-row items-center gap-3.5 ',
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
				<a
					href="{base}/user/workspaces"
					on:click={() => {
						localStorage.removeItem('workspace')
					}}
					class={twMerge(
						'text-primary flex gap-3.5 px-2 py-2 text-xs hover:bg-surface-hover hover:text-primary',
						'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
					)}
					role="menuitem"
					tabindex="-1"
					use:melt={item}
				>
					<Building size={14} />
					All workspaces
				</a>

				{#if $superadmin}
					<a
						href="#superadmin-settings"
						class={twMerge(
							'flex flex-row gap-3.5 items-center px-2 py-2 ',
							'text-secondary text-xs',
							'hover:bg-surface-hover hover:text-primary cursor-pointer',
							'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
						)}
						use:melt={item}
					>
						<ServerCog size={14} />
						Instance settings
					</a>
				{/if}

				<button
					on:click={() => logout()}
					class={twMerge(
						'flex flex-row gap-3.5  items-center px-2 py-2 w-full',
						'text-secondary text-xs',
						'hover:bg-surface-hover hover:text-primary cursor-pointer',
						'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
					)}
					use:melt={item}
				>
					<LogOut size={14} />
					Sign out
				</button>
			</div>
			<div
				on:mouseenter={() => (moreOpen = true)}
				on:mouseleave={() => (moreOpen = false)}
				role="none"
			>
				<div
					use:melt={item}
					on:m-focusin={() => (moreOpen = true)}
					on:m-focusout={() => (moreOpen = false)}
				>
					{#if !moreOpen || secondMenuLinks.length === 0}
						<div class="px-2 py-2 text-tertiary text-2xs">More...</div>
					{/if}
				</div>
				{#if moreOpen && secondMenuLinks.length > 0}
					{#each secondMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
						<div>
							<a
								href={menuLink.href}
								class={twMerge(
									'flex flex-row gap-3.5 items-center px-2 py-2 text-secondary text-2xs hover:bg-surface-hover hover:text-primary cursor-pointer',
									'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
								)}
								use:melt={item}
								on:m-focusin={() => (moreOpen = true)}
								on:m-focusout={() => (moreOpen = false)}
							>
								{menuLink.label}
							</a>
						</div>
					{/each}
				{/if}
			</div>
		</div>
		{#if $enterpriseLicense}
			<MultiplayerMenu />
		{/if}
	</Menu>
</Menubar>

<DarkModeObserver bind:darkMode />
