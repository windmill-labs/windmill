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
		Calendar,
		ServerCog,
		Table2,
		GraduationCap
	} from 'lucide-svelte'
	import SwitchWorkspaceSubmenu from './SwitchWorkspaceSubmenu.svelte'
	import {
		OPERATOR_MAIN_LINKS,
		OPERATOR_SECONDARY_LINKS,
		OPERATOR_TRIGGER_LINKS,
		type OperatorMenuLink,
		type OperatorTriggerLink
	} from './operatorRoutes'
	import { base } from '$lib/base'
	import { page } from '$app/state'
	import { TOUR_PARAM, TOUR_PARAM_VALUE } from '$lib/components/tutorials/operatorTour'

	import MultiplayerMenu from './MultiplayerMenu.svelte'
	import { Plus } from 'lucide-svelte'
	import {
		enterpriseLicense,
		superadmin,
		usedTriggerKinds,
		userWorkspaces,
		workspaceColor,
		workspaceStore
	} from '$lib/stores'
	import { twMerge } from 'tailwind-merge'
	import { getContrastTextColor } from '$lib/utils'
	import { USER_SETTINGS_HASH } from './settings'
	import { logout } from '$lib/logoutKit'
	import DarkModeObserver from '../DarkModeObserver.svelte'
	import BarsStaggered from '../icons/BarsStaggered.svelte'
	import { Menu, Menubar, MenuItem } from '$lib/components/meltComponents'
	import MenuButton, { sidebarClasses } from './MenuButton.svelte'
	import MenuLink from './MenuLink.svelte'
	import type { FavoriteKind } from './FavoriteMenu.svelte'
	let darkMode: boolean = $state(false)
	let showExtraTriggers = $state(false)
	// The menu opens on hover, so this doubles as "the pointer is on the button":
	// it slides the workspace name out beside the icon, and keeps it out once a
	// click has pinned the menu open and the pointer has moved away.
	let menuOpen = $state(false)

	interface Props {
		isCollapsed?: boolean
		favoriteLinks?: {
			label: string
			href: string
			kind: FavoriteKind
		}[]
	}

	let { isCollapsed = false, favoriteLinks = [] }: Props = $props()

	const MAIN_LINK_ICONS: Record<string, any> = { home: Home, runs: Play, schedules: Calendar }
	let activeWorkspace = $derived($userWorkspaces?.find((w) => w.id === $workspaceStore))
	let workspaceName = $derived(activeWorkspace?.name ?? $workspaceStore ?? '')
	// Home is where an operator lands and gets their bearings, so the name stays out there
	// rather than waiting for a hover. Everywhere else the button is the page's own chrome
	// and stays out of the way until asked.
	let showWorkspaceName = $derived(menuOpen || page.url.pathname === `${base}/`)
	// Most workspaces never get a colour (it is only ever set explicitly), and this button
	// floats over page content with nothing else behind it — so the disc falls back to a
	// neutral rather than vanishing, keeping the glyph on a ground of its own everywhere.
	// A token rather than a hex so it follows the theme; getContrastTextColor only parses
	// hex, so it returns undefined here and the glyph keeps its themed `text-hint`.
	const NEUTRAL_DISC = 'rgb(var(--color-surface-sunken))'

	let mainMenuLinks = $derived(
		OPERATOR_MAIN_LINKS.map((link) => ({ ...link, icon: MAIN_LINK_ICONS[link.id] })).filter(
			(link) => link.id === 'home' || filterLink(link)
		)
	)

	function filterLink(link: OperatorMenuLink) {
		if (!$userWorkspaces || !$workspaceStore) return false
		return activeWorkspace?.operator_settings?.[link.id] === true
	}
	let secondMenuLinks: OperatorMenuLink[] = $derived(OPERATOR_SECONDARY_LINKS.filter(filterLink))
	let allTriggerLinks: OperatorTriggerLink[] = $derived(OPERATOR_TRIGGER_LINKS.filter(filterLink))
	let secondMenuTriggerLinks = $derived(
		allTriggerLinks.filter((link) => $usedTriggerKinds.includes(link.kind))
	)
	let extraTriggerLinks = $derived(
		allTriggerLinks.filter((link) => !$usedTriggerKinds.includes(link.kind))
	)
</script>

<Menubar>
	{#snippet children({ createMenu })}
		<Menu
			{createMenu}
			placement="bottom-start"
			openOnHover
			usePointerDownOutside
			submenuSafe
			bind:open={menuOpen}
			on:close={() => (showExtraTriggers = false)}
		>
			{#snippet triggr({ trigger, pinned })}
				{@const iconColor = getContrastTextColor($workspaceColor)}
				<!-- Ground for the expanded state only: the button floats over page content and its
				     hover tint is translucent, so the name would otherwise sit on whatever is
				     underneath. Icon-only needs none — the disc is always drawn. -->
				<div class="flex rounded-md {showWorkspaceName ? 'bg-surface' : ''}">
					<MenuButton
						class="!text-xs"
						buttonClass={twMerge(
							'!pl-3.5 !pr-2 !w-auto',
							// A hover-opened menu leaves the button plain once the pointer moves on;
							// keeping the tint is what tells you the click pinned it.
							pinned ? sidebarClasses.selectedBg : ''
						)}
						icon={MenuIcon}
						isCollapsed={false}
						lightMode
						color={$workspaceColor ?? NEUTRAL_DISC}
						iconProps={iconColor ? { style: `color: ${iconColor}` } : undefined}
						label={showWorkspaceName ? workspaceName : undefined}
						ariaLabel={workspaceName ? `Menu — ${workspaceName}` : 'Menu'}
						{trigger}
					/>
				</div>
			{/snippet}
			{#snippet children({ item, builders })}
				<div class="w-full max-w-full">
					{#each favoriteLinks ?? [] as favorite (favorite.href)}
						<MenuItem
							href={favorite.href}
							{item}
							class={twMerge(
								'w-full inline-flex flex-row px-2 py-2',
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
								'transition-colors',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							lightMode
							{item}
						>
							<Settings size={14} />
							Account settings
						</MenuItem>

						<MenuItem
							href="{base}/?{TOUR_PARAM}={TOUR_PARAM_VALUE}"
							class={twMerge(
								'flex flex-row gap-3.5 items-center px-2 py-2',
								sidebarClasses.text,
								'transition-colors',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							lightMode
							{item}
						>
							<GraduationCap size={14} />
							Take the tour
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
								'transition-colors',
								sidebarClasses.text,
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
						<SwitchWorkspaceSubmenu {builders} {item} />

						{#if $superadmin}
							<MenuItem
								href="#superadmin-settings"
								class={twMerge(
									'flex flex-row gap-3.5 items-center px-2 py-2 ',
									'text-secondary text-xs',
									'cursor-pointer',
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
								'cursor-pointer',
								'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
							)}
							{item}
						>
							<LogOut size={14} />
							Sign out
						</MenuItem>
					</div>
					<div role="none">
						{#snippet renderSecondMenuLinks(menuLinks: OperatorMenuLink[])}
							{#each menuLinks as menuLink (menuLink.href ?? menuLink.label)}
								<MenuItem
									href={menuLink.href}
									class={twMerge(
										'flex flex-row gap-3.5 items-center px-2 py-2 text-secondary text-2xs cursor-pointer',
										'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
									)}
									{item}
								>
									{menuLink.label}
								</MenuItem>
							{/each}
						{/snippet}
						{#if secondMenuLinks.length || secondMenuTriggerLinks.length || extraTriggerLinks.length}
							<div class="divide-y">
								{#if secondMenuLinks.length}<div
										>{@render renderSecondMenuLinks(secondMenuLinks)}</div
									>{/if}
								{#if secondMenuTriggerLinks.length}<div
										>{@render renderSecondMenuLinks(secondMenuTriggerLinks)}</div
									>{/if}
								{#if extraTriggerLinks.length}<div>
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											role="none"
											onclickcapture={(e) => {
												// This row expands the list below it instead of acting on the selection, and
												// melt keeps the menu open only for a click it sees as defaultPrevented.
												// Svelte delegates onclick to the root, which runs after melt's own listener,
												// and a capture listener on the item itself would be ordered only by
												// registration, so an ancestor's capture phase is what reliably wins.
												e.preventDefault()
												showExtraTriggers = !showExtraTriggers
											}}
										>
											<MenuItem
												class={twMerge(
													'flex flex-row gap-3.5 items-center px-2 py-2 w-full text-secondary text-2xs cursor-pointer',
													'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
												)}
												{item}
											>
												<Plus size={12} />
												<span class="text-2xs">More triggers</span>
											</MenuItem>
										</div>
										{#if showExtraTriggers}
											{#each extraTriggerLinks as menuLink (menuLink.href)}
												<MenuItem
													href={menuLink.href}
													class={twMerge(
														'flex flex-row gap-3.5 items-center px-2 py-2 pl-6 text-tertiary text-2xs cursor-pointer',
														'data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
													)}
													{item}
												>
													{menuLink.label}
												</MenuItem>
											{/each}
										{/if}
									</div>{/if}
							</div>
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
