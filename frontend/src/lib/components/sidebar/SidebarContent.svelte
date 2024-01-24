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

	interface DatabaseStudio {
		slug: string
		version: string
		title: string
		tags: string[]
		description: string
	}

	function parseSimpleYAML(yamlString: string): DatabaseStudio {
		const result: Partial<DatabaseStudio> = {}
		const lines = yamlString.split('\n')

		for (const line of lines) {
			if (line.trim() === '---' || line.trim() === '') {
				continue
			}

			const [key, value] = line.split(':').map((part) => part.trim())

			if (key === 'tags' || key === 'features') {
				// Assuming the values are always in ['value1', 'value2'] format
				result[key] = value
					.slice(1, -1)
					.split(',')
					.map((tag) => tag.trim().replace(/^'(.+)'$/, '$1'))
			} else {
				result[key] = value
			}
		}

		return result as DatabaseStudio
	}

	async function fetchMarkdownFiles() {
		const apiUrl = `https://api.github.com/repos/windmill-labs/windmilldocs/contents/changelog?ref=main`
		const response = await fetch(apiUrl)
		const data = await response.json()

		if (!Array.isArray(data)) {
			return []
		}

		const directories = data.filter((x: { type: string }) => x.type === 'dir')

		const promises = directories.map((x: { url: string }) => fetch(x.url))
		const responses = await Promise.all(promises)
		const datas = await Promise.all(responses.map((x) => x.json()))

		const files = datas
			.flat()
			.filter((x) => x.name === 'index.md')
			.map((x) => x.download_url)

		const promises2 = files.map((x) => fetch(x))
		const responses2 = await Promise.all(promises2)
		const datas2 = await Promise.all(responses2.map((x) => x.text()))

		const changelogs = datas2.map((x) => parseSimpleYAML(x))

		console.log('changelogs', changelogs)

		localStorage.setItem('changelogs', JSON.stringify(changelogs))

		return changelogs
	}

	function getNewChangelogEntries(changelogs, storedChangelogs) {
		if (!storedChangelogs) {
			return changelogs
		}

		const newEntries = changelogs.filter((x) => {
			const storedChangelog = JSON.parse(storedChangelogs)
			const storedChangelogEntry = storedChangelog.find((y) => y.slug === x.slug)

			if (!storedChangelogEntry) {
				return true
			}

			return storedChangelogEntry.date
		})

		return newEntries
	}

	async function fetchAndStoreChangelogs() {
		// get lastVisit
		const lastVisit = localStorage.getItem('lastVisit')

		console.log('lastVisit', lastVisit)

		// only fetch once a day to avoid rate limit
		const today = new Date()
		const lastVisitDate = lastVisit ? new Date(lastVisit) : new Date(0)

		if (today.getDate() === lastVisitDate.getDate()) {
			console.log('no need to fetch changelogs')
			return
		}

		const changelogs = await fetchMarkdownFiles()
		const storedChangelogs = localStorage.getItem('changelogs')

		const newChangelogEntries = getNewChangelogEntries(changelogs, storedChangelogs)

		localStorage.setItem('lastVisit', today.toISOString())

		return newChangelogEntries
	}

	$: newChangelogs = fetchAndStoreChangelogs()

	$: console.log(newChangelogs)

	localStorage.setItem('lastVisit', '2023-01-01T00:00:00.000Z')

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
											{newChangelogs}
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
											<div class="absolute top-2 right-4 w-2 h-2 rounded-full bg-primary">
												<span class="relative flex h-3 w-3">
													<span
														class="animate-ping absolute inline-flex h-full w-full rounded-full bg-red-400 opacity-75"
													/>
													<span class="relative inline-flex rounded-full h-3 w-3 bg-red-500" />
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
