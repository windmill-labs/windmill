<script lang="ts">
	import {
		faBookOpen,
		faCalendar,
		faCode,
		faCubes,
		faEye,
		faHomeAlt,
		faPlay,
		faRobot,
		faUsersCog,
		faWallet,
		faWind,
		faCog
	} from '@fortawesome/free-solid-svg-icons'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import MenuLink from './MenuLink.svelte'
	import { userStore } from '$lib/stores'

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: faHomeAlt },
		{ label: 'Scripts', href: '/scripts', icon: faCode },
		{ label: 'Flows', href: '/flows', icon: faWind },
		{ label: 'Runs', href: '/runs', icon: faPlay },
		{ label: 'Schedules', href: '/schedules', icon: faCalendar },
		{ label: 'Variables', href: '/variables', icon: faWallet },
		{ label: 'Resources', href: '/resources', icon: faCubes }
	]

	$: secondaryMenuLinks = [
		{ label: 'Groups', href: '/groups', icon: faUsersCog },
		{ label: 'Audit Logs', href: '/audit_logs', icon: faEye },
		{
			label: 'Workspace',
			href: '/workspace_settings',
			icon: faCog,
			disabled: !$userStore?.is_admin
		},
		{ label: 'Workers', href: '/workers', icon: faRobot }
	]

	const thirdMenuLinks = [
		{ label: 'Documentation', href: 'https://docs.windmill.dev/docs/intro/', icon: faBookOpen },
		{ label: 'Feedbacks', href: 'https://discord.gg/V7PM2YHsPB', icon: faDiscord },
		{
			label: 'Issues',
			href: 'https://github.com/windmill-labs/windmill/issues/new',
			icon: faGithub
		}
	]

	export let isCollapsed: boolean = false
</script>

<div class="flex-1 flex flex-col py-4 overflow-x-hidden scrollbar-hidden">
	<nav class="h-full flex justify-between flex-col px-2">
		<div class="space-y-1">
			{#each mainMenuLinks as menuLink}
				<MenuLink class="text-lg" {...menuLink} {isCollapsed} />
			{/each}
			<div class="h-8" />
			{#each secondaryMenuLinks as menuLink}
				<MenuLink class="text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>

		<div class="space-1-2">
			<div class="h-4" />
			{#each thirdMenuLinks as menuLink}
				<MenuLink class="text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
	</nav>
</div>
