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
		faWallet,
		faCog,
		faStar
	} from '@fortawesome/free-solid-svg-icons'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import MenuLink from './MenuLink.svelte'
	import { starStore, userStore, workspaceStore } from '$lib/stores'
	import { FlowService, ScriptService } from '$lib/gen'

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: faHomeAlt },
		{ label: 'Runs', href: '/runs', icon: faPlay },
		{ label: 'Schedules', href: '/schedules', icon: faCalendar },
		{ label: 'Variables', href: '/variables', icon: faWallet },
		{ label: 'Resources', href: '/resources', icon: faCubes }
	]

	export let favoriteLinks = [] as { label: string; href: string }[]

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

<div class="flex-1 flex flex-col py-4 overflow-x-hidden scrollbar-hidden">
	<nav class="h-full flex justify-between flex-col px-2">
		<div class="space-y-1 pt-4">
			{#each mainMenuLinks as menuLink (menuLink.href)}
				<MenuLink class="text-lg" {...menuLink} {isCollapsed} />
			{/each}
			<div class="h-2" />
			<div class="max-h-40 overflow-y-auto flex flex-col space-y-1.5 max-w-xs">
				{#each favoriteLinks as menuLink (menuLink.href)}
					<MenuLink class="text-xs max-w-xs truncate" {...menuLink} {isCollapsed} icon={faStar} />
				{/each}
			</div>
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
