<script lang="ts">
	import MenuLink from './MenuLink.svelte'
	import { superadmin, userStore } from '$lib/stores'
	import { SIDEBAR_SHOW_SCHEDULES } from '$lib/consts'
	import {
		BookOpen,
		Bot,
		Boxes,
		Calendar,
		DollarSign,
		Eye,
		FolderCog,
		FolderOpen,
		Github,
		HelpCircle,
		Home,
		Play,
		ServerCog,
		Settings,
		UserCog
	} from 'lucide-svelte'
	import Menu from '../common/menu/MenuV2.svelte'
	import MenuButton from './MenuButton.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import UserMenu from './UserMenu.svelte'
	import DiscordIcon from '../icons/brands/Discord.svelte'

	$: mainMenuLinks = [
		{ label: 'Home', href: '/', icon: Home },
		{ label: 'Runs', href: '/runs', icon: Play },
		{ label: 'Variables', href: '/variables', icon: DollarSign, disabled: $userStore?.operator },
		{ label: 'Resources', href: '/resources', icon: Boxes, disabled: $userStore?.operator },
		{
			label: 'Schedules',
			href: '/schedules',
			icon: Calendar,
			disabled: !SIDEBAR_SHOW_SCHEDULES || $userStore?.operator
		}
	]

	$: secondaryMenuLinks = [
		// {
		// 	label: 'Workspace',
		// 	href: '/workspace_settings',
		// 	icon: Cog,
		// 	disabled: !$userStore?.is_admin && !$userStore?.is_super_admin
		// },
		{
			label: 'Settings',
			icon: Settings,
			subItems: [
				{
					label: 'Account',
					href: '#user-settings',
					icon: UserCog,
					faIcon: undefined
				},
				...($userStore?.is_admin || $superadmin
					? [
							{
								label: 'Workspace',
								href: '/workspace_settings',
								icon: FolderCog,
								faIcon: undefined
							}
					  ]
					: []),
				...($superadmin
					? [
							{
								label: 'Instance',
								href: '#superadmin-settings',
								icon: ServerCog,
								faIcon: undefined
							}
					  ]
					: [])
			],
			disabled: $userStore?.operator
		},
		{ label: 'Workers', href: '/workers', icon: Bot, disabled: $userStore?.operator },
		{
			label: 'Folders & Groups',
			icon: FolderOpen,
			subItems: [
				{
					label: 'Folders',
					href: '/folders',
					icon: FolderOpen,
					disabled: $userStore?.operator,
					faIcon: undefined
				},
				{
					label: 'Groups',
					href: '/groups',
					icon: UserCog,
					disabled: $userStore?.operator,
					faIcon: undefined
				}
			],
			disabled: $userStore?.operator
		},
		{ label: 'Audit Logs', href: '/audit_logs', icon: Eye, disabled: $userStore?.operator }
	]

	const thirdMenuLinks = [
		{
			label: 'Help',
			icon: HelpCircle,
			subItems: [
				{ label: 'Docs', href: 'https://www.windmill.dev/docs/intro/', icon: BookOpen },
				{
					label: 'Feedbacks',
					href: 'https://discord.gg/V7PM2YHsPB',
					icon: DiscordIcon
				},
				{
					label: 'Issues',
					href: 'https://github.com/windmill-labs/windmill/issues/new',
					icon: Github
				}
			]
		}
	]

	export let isCollapsed: boolean = false
</script>

<nav
	class="grow flex flex-col overflow-x-hidden scrollbar-hidden px-2 md:pb-2 gap-16 justify-between"
>
	<div class="space-y-1 pt-4 mb-6 md:mb-10">
		{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
			<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
		{/each}
	</div>
	<div class="flex flex-col h-full justify-end">
		<div class="space-y-0.5 mb-6 md:mb-10">
			<UserMenu {isCollapsed} />
			{#each secondaryMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
				{#if menuLink.subItems}
					<Menu>
						<div slot="trigger">
							<MenuButton class="!text-2xs" {...menuLink} {isCollapsed} />
						</div>
						{#each menuLink.subItems as subItem (subItem.href ?? subItem.label)}
							<MenuItem>
								<div class="py-1" role="none">
									<a
										href={subItem.href}
										class="text-secondary block px-4 py-2 text-2xs hover:bg-surface-hover hover:text-primary"
										role="menuitem"
										tabindex="-1"
									>
										<div class="flex flex-row items-center gap-2">
											{#if subItem.icon}
												<svelte:component this={subItem.icon} size={16} />
											{/if}

											{subItem.label}
										</div>
									</a>
								</div>
							</MenuItem>
						{/each}
					</Menu>
				{:else}
					<MenuLink class="!text-2xs" {...menuLink} {isCollapsed} />
				{/if}
			{/each}
		</div>
		<div class="space-y-0.5">
			{#each thirdMenuLinks as menuLink (menuLink)}
				{#if menuLink.subItems}
					<Menu>
						<div slot="trigger">
							<MenuButton class="!text-2xs" {...menuLink} {isCollapsed} />
						</div>
						{#each menuLink.subItems as subItem (subItem.href ?? subItem.label)}
							<MenuItem>
								<div class="py-1" role="none">
									<a
										href={subItem.href}
										class="text-secondary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary"
										role="menuitem"
										tabindex="-1"
										target="_blank"
									>
										<div class="flex flex-row items-center gap-2">
											{#if subItem.icon}
												<svelte:component this={subItem.icon} size={16} />
											{/if}

											{subItem.label}
										</div>
									</a>
								</div>
							</MenuItem>
						{/each}
					</Menu>
				{/if}
			{/each}
		</div>
	</div>
</nav>
