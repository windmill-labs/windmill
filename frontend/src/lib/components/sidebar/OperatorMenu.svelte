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
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton from './MenuButton.svelte'
	import MenuLink from './MenuLink.svelte'
	import { onDestroy } from 'svelte'
	let darkMode: boolean = $state(false)

	interface Props {
		isCollapsed?: boolean
		favoriteLinks?: any
	}

	let {
		isCollapsed = false,
		favoriteLinks = [] as {
			label: string
			href: string
			kind: 'script' | 'flow' | 'app' | 'raw_app'
		}[]
	}: Props = $props()

	let mainMenuLinks = $derived(
		[
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
	)

	let secondMenuLinks = $derived(
		[
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
				label: 'Assets',
				id: 'assets',
				href: `${base}/assets`
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
				label: 'SQS triggers',
				id: 'triggers',
				href: `${base}/sqs_triggers`
			},
			{
				label: 'GCP Pub/Sub triggers',
				id: 'triggers',
				href: `${base}/gcp_triggers`
			},
			{
				label: 'MQTT triggers',
				id: 'triggers',
				href: `${base}/mqtt_triggers`
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
	)

	let moreOpen = $state(false)
	let moreOpenTimeout: NodeJS.Timeout | undefined = $state()

	function debouncedSetMoreOpen(value: boolean) {
		if (moreOpenTimeout) {
			clearTimeout(moreOpenTimeout)
		}
		moreOpenTimeout = setTimeout(() => {
			moreOpen = value
		}, 150) // 150ms debounce
	}

	onDestroy(() => {
		if (moreOpenTimeout) {
			clearTimeout(moreOpenTimeout)
		}
	})
</script>

<Menubar>
	{#snippet children({ createMenu })}
		<Menu {createMenu} usePointerDownOutside>
			{#snippet triggr({ trigger })}
				<MenuButton
					class="!text-xs"
					icon={MenuIcon}
					{isCollapsed}
					lightMode
					label={undefined}
					{trigger}
				/>
			{/snippet}
			{#snippet children({ item })}
				<div class="w-full max-w-full">
					{#each favoriteLinks ?? [] as favorite (favorite.href)}
						<MenuItem
							href={favorite.href}
							{item}
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
						</MenuItem>
					{/each}
				</div>

				{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
					<MenuLink class="!text-xs" {...menuLink} {isCollapsed} lightMode {item} />
				{/each}

				<div class="divide-y" role="none">
					<div role="none">
						<MenuItem
							href={USER_SETTINGS_HASH}
							class={twMerge(
								'flex flex-row gap-3.5 items-center px-2 py-2 ',
								'text-secondary text-xs',
								'hover:bg-surface-hover hover:text-primary cursor-pointer',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							<Settings size={14} />
							Account settings
						</MenuItem>
					</div>

					<div role="none">
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
							class={twMerge(
								'text-secondary block text-left px-2 py-2 font-normal text-xs hover:bg-surface-hover hover:text-primary w-full',
								'flex flex-row items-center gap-3.5 ',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							{#if darkMode}
								<Sun size={14} />
							{:else}
								<Moon size={14} />
							{/if}
							Switch theme
						</MenuItem>
						<MenuItem
							href="{base}/user/workspaces"
							onClick={() => {
								localStorage.removeItem('workspace')
							}}
							class={twMerge(
								'text-primary flex gap-3.5 px-2 py-2 text-xs hover:bg-surface-hover hover:text-primary',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							<Building size={14} />
							All workspaces
						</MenuItem>

						{#if $superadmin}
							<MenuItem
								href="#superadmin-settings"
								class={twMerge(
									'flex flex-row gap-3.5 items-center px-2 py-2 ',
									'text-secondary text-xs',
									'hover:bg-surface-hover hover:text-primary cursor-pointer',
									'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
								)}
								{item}
							>
								<ServerCog size={14} />
								Instance settings
							</MenuItem>
						{/if}

						<MenuItem
							onClick={() => logout()}
							class={twMerge(
								'flex flex-row gap-3.5  items-center px-2 py-2 w-full',
								'text-secondary text-xs',
								'hover:bg-surface-hover hover:text-primary cursor-pointer',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							<LogOut size={14} />
							Sign out
						</MenuItem>
					</div>
					<div
						onmouseenter={() => debouncedSetMoreOpen(true)}
						onmouseleave={() => debouncedSetMoreOpen(false)}
						role="none"
					>
						<MenuItem
							onFocusIn={() => debouncedSetMoreOpen(true)}
							onFocusOut={() => debouncedSetMoreOpen(false)}
							{item}
						>
							{#if !moreOpen || secondMenuLinks.length === 0}
								<div class="px-2 py-2 text-tertiary text-2xs">More...</div>
							{/if}
						</MenuItem>
						{#if moreOpen && secondMenuLinks.length > 0}
							{#each secondMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
								<div>
									<MenuItem
										href={menuLink.href}
										class={twMerge(
											'flex flex-row gap-3.5 items-center px-2 py-2 text-secondary text-2xs hover:bg-surface-hover hover:text-primary cursor-pointer',
											'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
										)}
										{item}
										onFocusIn={() => debouncedSetMoreOpen(true)}
										onFocusOut={() => debouncedSetMoreOpen(false)}
									>
										{menuLink.label}
									</MenuItem>
								</div>
							{/each}
						{/if}
					</div>
				</div>
				{#if $enterpriseLicense}
					<div
						onmouseenter={() => {
							if (moreOpenTimeout) {
								setTimeout(() => {
									clearTimeout(moreOpenTimeout)
								}, 15)
							}
						}}
						onmouseleave={() => {
							debouncedSetMoreOpen(false)
						}}
						role="none"
					>
						<MultiplayerMenu />
					</div>
				{/if}
			{/snippet}
		</Menu>
	{/snippet}
</Menubar>

<DarkModeObserver bind:darkMode />
