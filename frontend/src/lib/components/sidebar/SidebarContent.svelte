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
		faDollarSign
	} from '@fortawesome/free-solid-svg-icons'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import MenuLink from './MenuLink.svelte'
	import { superadmin, userStore } from '$lib/stores'

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: faHomeAlt },
		{ label: 'Runs', href: '/runs', icon: faPlay },
		{ label: 'Variables', href: '/variables', icon: faDollarSign, disabled: $userStore?.operator },
		{ label: 'Resources', href: '/resources', icon: faCubes, disabled: $userStore?.operator }
	]

	$: secondaryMenuLinks = [
		{ label: 'Schedules', href: '/schedules', icon: faCalendar, disabled: $userStore?.operator },
		{ label: 'Groups', href: '/groups', icon: faUsersCog, disabled: $userStore?.operator },
		{ label: 'Audit Logs', href: '/audit_logs', icon: faEye, disabled: $userStore?.operator },
		{
			label: 'Workspace',
			href: '/workspace_settings',
			icon: faCog,
			disabled: !($userStore?.is_admin || $superadmin)
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

<div class="flex-1 flex flex-col pb-4 overflow-x-hidden scrollbar-hidden">
	<nav class="h-full flex justify-between flex-col px-2">
		<div class="space-y-1 pt-4">
			{#each mainMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="text-md" {...menuLink} {isCollapsed} />
			{/each}
			<div class="h-2" />
		</div>
		<div class="space-1-2">
			{#each secondaryMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="text-xs" {...menuLink} {isCollapsed} />
			{/each}
			<div class="h-8" />
			{#each thirdMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
	</nav>
</div>
