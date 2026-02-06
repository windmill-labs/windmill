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
		ServerCog,
		GraduationCap,
		Table2
	} from 'lucide-svelte'
	import { base } from '$lib/base'

	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import {
		clearWorkspaceFromStorage,
		enterpriseLicense,
		superadmin,
		userWorkspaces,
		workspaceStore,
		tutorialsToDo,
		skippedAll
	} from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { USER_SETTINGS_HASH } from './settings'
	import { logout } from '$lib/logoutKit'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import BarsStaggered from '../icons/BarsStaggered.svelte'
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton, { sidebarClasses } from './MenuButton.svelte'
	import MenuLink from './MenuLink.svelte'
	import ResizeTransitionWrapper from '../common/ResizeTransitionWrapper.svelte'
	import type { FavoriteKind } from './FavoriteMenu.svelte'
	let darkMode: boolean = $state(false)

	interface Props {
		isCollapsed?: boolean
		favoriteLinks?: {
			label: string
			href: string
			kind: FavoriteKind
		}[]
	}

	let { isCollapsed = false, favoriteLinks = [] }: Props = $props()

	let mainMenuLinks = $derived(
		[
			{ label: 'Home', id: 'home', href: `${base}/`, icon: Home },
			{ label: 'Runs', id: 'runs', href: `${base}/runs`, icon: Play },
			{ label: 'Schedules', id: 'schedules', href: `${base}/schedules`, icon: Calendar },
			// Add Tutorials to main menu only if not all completed and not skipped
			...($tutorialsToDo.length > 0 && !$skippedAll
				? [
						{
							label: 'Tutorials',
							id: 'tutorials',
							href: `${base}/tutorials`,
							icon: GraduationCap
						}
					]
				: [])
		].filter(
			(link) =>
				link.id === 'home' ||
				link.id === 'tutorials' ||
				($userWorkspaces &&
					$workspaceStore &&
					$userWorkspaces.find((_) => _.id === $workspaceStore)?.operator_settings?.[link.id] ===
						true)
		)
	)

	type SecondMenuLink = { label: string; id: string; href: string }
	function filterLink(link: SecondMenuLink) {
		if (!$userWorkspaces || !$workspaceStore) return false
		let userWorkspace = $userWorkspaces.find((_) => _.id === $workspaceStore)
		return userWorkspace?.operator_settings?.[link.id] === true
	}
	let secondMenuLinks: SecondMenuLink[] = $derived(
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
		].filter(filterLink)
	)
	let secondMenuTriggerLinks = $derived(
		[
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
				label: 'Email triggers',
				id: 'triggers',
				href: `${base}/email_triggers`
			},
			{
				label: 'Audit logs',
				id: 'audit_logs',
				href: `${base}/audit_logs`
			}
		].filter(filterLink)
	)
	let showMore = $state(false)
</script>

<Menubar>
	{#snippet children({ createMenu })}
		<Menu {createMenu} usePointerDownOutside on:close={() => (showMore = false)}>
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
								{:else if favorite.kind == 'asset'}
									<Table2 size={16} />
								{/if}
							</span>
							<span class="text-primary ml-2 grow min-w-0 text-xs truncate">
								{favorite.label}
							</span>
						</MenuItem>
					{/each}
				</div>

				{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
					<MenuLink class="!text-xs" {...menuLink} {isCollapsed} {item} lightMode />
				{/each}

				<div class="divide-y" role="none">
					<div role="none">
						<MenuItem
							href={USER_SETTINGS_HASH}
							class={twMerge(
								'flex flex-row gap-3.5 items-center px-2 py-2',
								sidebarClasses.text,
								sidebarClasses.hoverBg
							)}
							lightMode
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
							lightMode
							class={twMerge(
								'w-full flex gap-3.5 px-2 py-2',
								sidebarClasses.hoverBg,
								sidebarClasses.text
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
							onClick={() => clearWorkspaceFromStorage()}
							lightMode
							class={twMerge('flex gap-3.5 px-2 py-2', sidebarClasses.hoverBg, sidebarClasses.text)}
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
								'text-primary text-xs',
								'hover:bg-surface-hover cursor-pointer',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							<LogOut size={14} />
							Sign out
						</MenuItem>
					</div>
					<div onmouseleave={() => (showMore = false)} role="none">
						{#if secondMenuLinks.length}
							<ResizeTransitionWrapper vertical innerClass="w-full">
								{#if !showMore}
									<div onmouseenter={() => (showMore = true)} role="none">
										<MenuItem {item}>
											<div class="px-2 py-2 text-primary text-2xs">More...</div>
										</MenuItem>
									</div>
								{:else}
									{#snippet renderSecondMenuLinks(menuLinks: SecondMenuLink[])}
										{#each menuLinks as menuLink (menuLink.href ?? menuLink.label)}
											<MenuItem
												href={menuLink.href}
												class={twMerge(
													'flex flex-row gap-3.5 items-center px-2 py-2 text-secondary text-2xs hover:bg-surface-hover hover:text-primary cursor-pointer',
													'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
												)}
												{item}
											>
												{menuLink.label}
											</MenuItem>
										{/each}
									{/snippet}
									<div class="divide-y">
										<div>{@render renderSecondMenuLinks(secondMenuLinks)}</div>
										<div>{@render renderSecondMenuLinks(secondMenuTriggerLinks)}</div>
									</div>
								{/if}
							</ResizeTransitionWrapper>
						{/if}
						{#if $enterpriseLicense}
							<MultiplayerMenu />
						{/if}
					</div>
				</div>
			{/snippet}
		</Menu>
	{/snippet}
</Menubar>

<DarkModeObserver bind:darkMode />
