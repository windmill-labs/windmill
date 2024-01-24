<script lang="ts">
	import MenuLink from './MenuLink.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
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
		LogOut,
		Newspaper,
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
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clearStores } from '$lib/storeUtils'
	import { goto } from '$app/navigation'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import { onMount } from 'svelte'

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

	async function leaveWorkspace() {
		await WorkspaceService.leaveWorkspace({ workspace: $workspaceStore ?? '' })
		sendUserToast('You left the workspace')
		clearStores()
		goto('/user/workspaces')
	}

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
					: []),
				...(!$superadmin && !$userStore?.is_admin
					? [
							{
								label: 'Leave Workspace',
								action: () => {
									leaveWorkspaceModal = true
								},
								class: 'text-red-400',
								icon: LogOut,
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

	const changelogs = [
		{
			label: 'Ag Charts',
			description:
				'The Ag Charts component integrates the Ag Charts library, enabling the visualization of data through various chart types. This component is designed to offer a flexible and powerful way to display data graphically within the application.',
			href: 'https://www.windmill.dev/changelog/ag-charts',
			date: '2024-01-23'
		},
		{
			label: 'Database Studio',
			description:
				"Introducing the Database Studio, a web-based database management tool that leverages Ag Grid for table display and interaction. This component enhances the user's ability to interact with database content in a dynamic and intuitive way.",
			href: 'https://www.windmill.dev/changelog/database-studio',
			date: '2024-01-23'
		},
		{
			label: 'Rich results render',
			description: 'Added rich results render for arrays of objects and markdown.',
			href: 'https://www.windmill.dev/changelog/rich-render',
			date: '2024-01-23'
		}
	]

	let newChangelogs: {
		label: string
		description: string
		href: string
		date: string
	}[] = []

	onMount(() => {
		newChangelogs = changelogs.filter((c) => {
			const lastVisit = localStorage.getItem('lastVisit')
			return lastVisit ? new Date(c.date) > new Date(lastVisit) : true
		})

		localStorage.setItem('lastVisit', new Date().toISOString())
	})

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
				},
				{
					label: 'Changelog',
					href: 'https://www.windmill.dev/changelog/',
					icon: Newspaper
				}
			]
		}
	]

	export let isCollapsed: boolean = false
	export let noGap: boolean = false

	let leaveWorkspaceModal = false
</script>

<nav
	class={twMerge(
		'grow flex flex-col overflow-x-hidden scrollbar-hidden px-2 md:pb-2 justify-between',
		noGap ? 'gap-0' : 'gap-16'
	)}
>
	<div class={twMerge('space-y-1 pt-4 ', noGap ? 'md:mb-0 mb-0' : 'mb-6 md:mb-10')}>
		{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
			<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
		{/each}
	</div>
	<div class="flex flex-col h-full justify-end">
		<div class={twMerge('space-y-0.5 mb-6 md:mb-10', noGap ? 'md:mb-0 mb-0' : 'mb-6 md:mb-10')}>
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
									{#if subItem?.['action']}
										<button
											class="text-secondary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary"
											on:click={subItem?.['action']}
										>
											<div class="flex flex-row items-center gap-2">
												{#if subItem.icon}
													<svelte:component this={subItem.icon} size={16} />
												{/if}

												{subItem.label}
											</div>
										</button>
									{:else}
										<a
											href={subItem.href}
											class={twMerge(
												'text-secondary block px-4 py-2 text-2xs hover:bg-surface-hover hover:text-primary'
											)}
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
									{/if}
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
										class="text-secondary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary relative"
										role="menuitem"
										tabindex="-1"
										target="_blank"
									>
										{#if newChangelogs.length > 0 && subItem.label === 'Changelog'}
											<div class="absolute top-3 right-4 w-3 h-3 rounded-full bg-primary">
												<span class="relative flex h-3 w-3">
													<span
														class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"
													/>
													<span class="relative inline-flex rounded-full h-3 w-3 bg-blue-500" />
												</span>
											</div>
										{/if}
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
						{#if newChangelogs.length > 0}
							<div class="w-full h-1 border-t" />
							<span class="text-xs px-4 font-bold"> Latest changelogs </span>
							{#each newChangelogs as changelog}
								<MenuItem>
									<div class="py-1" role="none">
										<a
											href={changelog.href}
											class="text-secondary block px-4 py-2 text-xs hover:bg-surface-hover hover:text-primary relative"
											role="menuitem"
											tabindex="-1"
											target="_blank"
										>
											<div class="flex flex-row items-center gap-2">
												{changelog.label}
											</div>
										</a>
									</div>
								</MenuItem>
							{/each}
						{/if}
					</Menu>
				{/if}
			{/each}
		</div>
	</div>
</nav>

<ConfirmationModal
	open={leaveWorkspaceModal}
	title="Leave workspace"
	confirmationText="Remove"
	on:canceled={() => {
		leaveWorkspaceModal = false
	}}
	on:confirmed={() => {
		leaveWorkspace()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to leave this workspace?</span>
	</div>
</ConfirmationModal>
