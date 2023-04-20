<script lang="ts">
	import {
		faBookOpen,
		faCalendar,
		faCubes,
		faEye,
		faHomeAlt,
		faPlay,
		faRobot,
		faUsersCog,
		faCog,
		faDollarSign,
		faFolderOpen
	} from '@fortawesome/free-solid-svg-icons'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import MenuLink from './MenuLink.svelte'
	import { userStore } from '$lib/stores'
	import { SIDEBAR_SHOW_SCHEDULES } from '$lib/consts'

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: faHomeAlt },
		{ label: 'Runs', href: '/runs', icon: faPlay },
		{ label: 'Variables', href: '/variables', icon: faDollarSign, disabled: $userStore?.operator },
		{ label: 'Resources', href: '/resources', icon: faCubes, disabled: $userStore?.operator }
	]

	$: secondaryMenuLinks = [
		{ label: 'Schedules', href: '/schedules', icon: faCalendar, disabled: !SIDEBAR_SHOW_SCHEDULES || $userStore?.operator },
		{ label: 'Folders', href: '/folders', icon: faFolderOpen, disabled: $userStore?.operator },
		{ label: 'Groups', href: '/groups', icon: faUsersCog, disabled: $userStore?.operator },
		{ label: 'Audit Logs', href: '/audit_logs', icon: faEye, disabled: $userStore?.operator },
		{
			label: 'Workspace',
			href: '/workspace_settings',
			icon: faCog,
			disabled: !$userStore?.is_admin && !$userStore?.is_super_admin
		},
		{ label: 'Workers', href: '/workers', icon: faRobot, disabled: $userStore?.operator }
	]

	const thirdMenuLinks = [
		{ label: 'Docs', href: 'https://docs.windmill.dev/docs/intro/', icon: faBookOpen },
		{ label: 'Feedbacks', href: 'https://discord.gg/V7PM2YHsPB', icon: faDiscord },
		{
			label: 'Issues',
			href: 'https://github.com/windmill-labs/windmill/issues/new',
			icon: faGithub
		}
	]

	export let isCollapsed: boolean = false
</script>

<nav class="grow flex md:justify-between flex-col overflow-x-hidden scrollbar-hidden px-2 md:pb-4">
	<div class="space-y-1 pt-4 mb-6 md:mb-10">
		{#each mainMenuLinks as menuLink (menuLink.href)}
			<MenuLink class="!text-md" {...menuLink} {isCollapsed} />
		{/each}
	</div>
	<div>
		<div class="space-y-0.5 mb-6 md:mb-10">
			{#each secondaryMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
		<div class="space-y-0.5">
			{#each thirdMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
	</div>
</nav>
