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
		faWind
	} from '@fortawesome/free-solid-svg-icons'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import MenuLink from './MenuLink.svelte'

	const mainMenuLinks = [
		{ label: 'Home', href: '/', icon: faHomeAlt },
		{ label: 'Scripts', href: '/scripts', icon: faCode },
		{ label: 'Flows', href: '/flows', icon: faWind },
		{ label: 'Runs', href: '/runs', icon: faPlay },
		{ label: 'Schedules', href: '/schedules', icon: faCalendar },
		{ label: 'Variables', href: '/variables', icon: faWallet },
		{ label: 'Resources', href: '/resources', icon: faCubes }
	]

	const secondaryMenuLinks = [
		{ label: 'Workers', href: '/workers', icon: faRobot },
		{ label: 'Groups', href: '/groups', icon: faUsersCog },
		{ label: 'Audit Logs', href: '/audit_logs', icon: faEye }
	]

	const thirdMenuLinks = [
		{ label: 'Documentation', href: 'https://docs.windmill.dev/docs/intro/', icon: faBookOpen },
		{ label: 'Feedbacks?', href: 'https://discord.gg/V7PM2YHsPB', icon: faDiscord },
		{
			label: 'Issues?',
			href: 'https://github.com/windmill-labs/windmill/issues/new',
			icon: faGithub
		}
	]

	export let isCollapsed: boolean = false
</script>

<div class="flex-1 flex flex-col pt-5 pb-4 overflow-y-auto overflow-x-hidden">
	<div class="flex items-center flex-shrink-0 px-4 font-extrabold text-white">
		{#if isCollapsed}
			W
		{:else}
			Windmill
		{/if}
	</div>

	<nav class="mt-5 flex-1 px-2 space-y-8">
		<div class="space-y-2">
			{#each mainMenuLinks as menuLink}
				<MenuLink {...menuLink} {isCollapsed} />
			{/each}
		</div>

		<div class="border-b border-gray-300" />

		<div class="space-y-2">
			{#each secondaryMenuLinks as menuLink}
				<MenuLink {...menuLink} {isCollapsed} />
			{/each}
		</div>
		<div class="border-b border-gray-300" />

		<div class="space-y-2">
			{#each thirdMenuLinks as menuLink}
				<MenuLink {...menuLink} {isCollapsed} />
			{/each}
		</div>
	</nav>
</div>
