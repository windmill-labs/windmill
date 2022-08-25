<script lang="ts">
	import MenuLink from '$lib/components/sidebar/MenuLink.svelte'
	import { OpenAPI } from '$lib/gen'
	import { logout } from '$lib/logout'
	import { superadmin, userStore, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import { classNames, clickOutside } from '$lib/utils'
	import { faDiscord, faGithub } from '@fortawesome/free-brands-svg-icons'
	import {
		faBookOpen,
		faCalendar,
		faCode,
		faCrown,
		faCog,
		faCubes,
		faEye,
		faHomeAlt,
		faPlay,
		faRobot,
		faUsersCog,
		faWallet,
		faWind,
		faArrowLeft
	} from '@fortawesome/free-solid-svg-icons'
	import { onMount } from 'svelte'
	import Icon from 'svelte-awesome'
	import '../app.css'

	OpenAPI.WITH_CREDENTIALS = true

	let menuOpen = false
	let workspacePickerOpen = false
	let isMobile = false
	let viewportWidth = 3000
	let isCollapsed = false

	function openMenu(): void {
		menuOpen = true
	}

	function handleClickOutside(event: any): void {
		if (isMobile || viewportWidth < 640) {
			isCollapsed = true
		}
	}

	function handleClickOutsideMenu(event: any): void {
		menuOpen = false
	}
	function handleClickOutsideWorkspacePicker(event: any): void {
		workspacePickerOpen = false
	}

	onMount(async () => {
		isMobile = /iPhone|iPad|iPod|Android/i.test(navigator.userAgent)
		//Mobile
		isCollapsed = isMobile
	})

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
		{ label: 'Feedback', href: 'https://discord.gg/V7PM2YHsPB', icon: faDiscord },
		{
			label: 'Issues',
			href: 'https://github.com/windmill-labs/windmill/issues/new',
			icon: faGithub
		}
	]

	const selectedIndex = 1
</script>

<div>
	<div class={classNames('relative z-40 md:hidden')} role="dialog" aria-modal="true">
		<div
			class={classNames(
				'fixed inset-0 bg-gray-600 bg-opacity-75 transition-opacity ease-linear duration-300',
				menuOpen ? 'opacity-100' : 'opacity-0'
			)}
		/>

		<div class="fixed inset-0 flex z-40">
			<div
				class={classNames(
					'relative flex-1 flex flex-col max-w-xs w-full bg-white transition ease-in-out duration-300 transform',
					menuOpen ? 'translate-x-0' : '-translate-x-full'
				)}
			>
				<div
					class={classNames(
						'absolute top-0 right-0 -mr-12 pt-2 ease-in-out duration-300',
						menuOpen ? 'opacity-100' : 'opacity-0'
					)}
				>
					<button
						type="button"
						on:click={() => {
							menuOpen = !menuOpen
						}}
						class="ml-1 flex items-center justify-center h-10 w-10 rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white"
					>
						<span class="sr-only">Close sidebar</span>
						<!-- Heroicon name: outline/x -->
						<svg
							class="h-6 w-6 text-white"
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="2"
							stroke="currentColor"
							aria-hidden="true"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<div class="flex-1 h-0 pt-5 pb-4 overflow-y-auto">
					<div class="flex-shrink-0 flex items-center px-4">
						<img
							class="h-8 w-auto"
							src="https://docs.windmill.dev/img/windmill.svg"
							alt="Windmill"
						/>
					</div>
					<nav class="mt-5 px-2 space-y-2">
						{#each mainMenuLinks as menuLink, index}
							<MenuLink {...menuLink} {isCollapsed} />
						{/each}
						<div class="border-b" />
						{#each secondaryMenuLinks as menuLink, index}
							<MenuLink {...menuLink} {isCollapsed} />
						{/each}
						<div class="border-b" />
						{#each thirdMenuLinks as menuLink, index}
							<MenuLink {...menuLink} {isCollapsed} />
						{/each}
					</nav>
				</div>
			</div>
		</div>
	</div>

	<!-- Static sidebar for desktop -->
	<div
		class={classNames(
			'hidden md:flex md:flex-col md:fixed md:inset-y-0 transition-all ease-in-out duration-200',
			isCollapsed ? 'md:w-12' : 'md:w-64'
		)}
	>
		<div class="flex-1 flex flex-col min-h-0 border-r border-gray-200 bg-white">
			<div class="flex-1 flex flex-col pt-5 pb-4 overflow-y-auto overflow-x-hidden">
				<div class="flex items-center flex-shrink-0 px-4 justify-between">
					<img class="h-8 w-auto" src="https://docs.windmill.dev/img/windmill.svg" alt="Windmill" />
				</div>

				<nav class="mt-5 flex-1 px-2 bg-white space-y-2">
					{#each mainMenuLinks as menuLink, index}
						<MenuLink {...menuLink} {isCollapsed} />
					{/each}
					<div class="border-b" />
					{#each secondaryMenuLinks as menuLink, index}
						<MenuLink {...menuLink} {isCollapsed} />
					{/each}
					<div class="border-b" />
					{#each thirdMenuLinks as menuLink, index}
						<MenuLink {...menuLink} {isCollapsed} />
					{/each}
				</nav>
			</div>

			<div class="flex-shrink-0 flex border-t border-gray-200 p-4">
				<button
					on:click={() => {
						isCollapsed = !isCollapsed
					}}
				>
					<Icon
						data={faArrowLeft}
						class={classNames(
							'flex-shrink-0 h-4 w-4 transition-all ease-in-out duration-200',
							isCollapsed ? 'rotate-180' : 'rotate-0'
						)}
					/>
				</button>
			</div>
		</div>
	</div>
	<div
		class={classNames(
			'flex flex-col flex-1 transition-all ease-in-out duration-200',
			isCollapsed ? 'md:pl-12' : 'md:pl-64'
		)}
	>
		<main>
			<div class="p-2 border-b flex justify-between flex-row-reverse">
				<div>asopd</div>
				<div class="md:hidden">
					<button
						type="button"
						on:click={() => {
							menuOpen = false
						}}
						class="-ml-0.5 -mt-0.5 h-12 w-12 inline-flex items-center justify-center rounded-md text-gray-500 hover:text-gray-900 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-indigo-500"
					>
						<span class="sr-only">Open sidebar</span>
						<!-- Heroicon name: outline/menu -->
						<svg
							class="h-6 w-6"
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="2"
							stroke="currentColor"
							aria-hidden="true"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M4 6h16M4 12h16M4 18h16" />
						</svg>
					</button>
				</div>
			</div>
			<slot />
		</main>
	</div>
</div>
