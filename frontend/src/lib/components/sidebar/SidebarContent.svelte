<script lang="ts">
	import MenuLink from './MenuLink.svelte'
	import { userStore } from '$lib/stores'
	import { SIDEBAR_SHOW_SCHEDULES } from '$lib/consts'
	import {
		BookOpen,
		Boxes,
		Calendar,
		Cog,
		DollarSign,
		Eye,
		FolderOpen,
		HelpCircle,
		Home,
		Play,
		ServerCog,
		UserCog
	} from 'lucide-svelte'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import Menu from '../common/menu/MenuV2.svelte'
	import MenuButton from './MenuButton.svelte'

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
		{ label: 'Folders', href: '/folders', icon: FolderOpen, disabled: $userStore?.operator },
		{ label: 'Groups', href: '/groups', icon: UserCog, disabled: $userStore?.operator },
		{ label: 'Audit Logs', href: '/audit_logs', icon: Eye, disabled: $userStore?.operator },
		{
			label: 'Workspace',
			href: '/workspace_settings',
			icon: Cog,
			disabled: !$userStore?.is_admin && !$userStore?.is_super_admin
		},
		{ label: 'Workers', href: '/workers', icon: ServerCog, disabled: $userStore?.operator }
	]

	const thirdMenuLinks = [
		{ label: 'Docs', href: 'https://www.windmill.dev/docs/intro/', icon: BookOpen },
		{
			label: 'Help',
			icon: HelpCircle,
			subItems: [
				{
					label: 'Feedbacks',
					href: 'https://discord.gg/V7PM2YHsPB',
					faIcon: faDiscord
				},
				{
					label: 'Issues',
					href: 'https://github.com/windmill-labs/windmill/issues/new',
					faIcon: faGithub
				}
			]
		}
	]

	export let isCollapsed: boolean = false
</script>

<nav class="grow flex flex-col overflow-x-hidden scrollbar-hidden px-2 md:pb-4">
	<div class="space-y-1 pt-4 mb-6 md:mb-10">
		{#each mainMenuLinks as menuLink (menuLink.href)}
			<MenuLink class="!text-sm" {...menuLink} {isCollapsed} />
		{/each}
	</div>
	<div class="flex flex-col h-full justify-between">
		<div class="space-y-0.5 mb-6 md:mb-10">
			{#each secondaryMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
		<div class="space-y-0.5">
			{#each thirdMenuLinks as menuLink (menuLink.href)}
				{#if menuLink.subItems}
					<Menu>
						<div slot="trigger">
							<MenuButton class="!text-xs" {...menuLink} {isCollapsed} />
						</div>
						{#each menuLink.subItems as subItem (subItem.href)}
							<MenuLink class="!text-xs" {...subItem} {isCollapsed} />
						{/each}
					</Menu>
				{:else}
					<MenuLink class="!text-xs z-50" {...menuLink} {isCollapsed} />
				{/if}
			{/each}
		</div>
	</div>
</nav>
