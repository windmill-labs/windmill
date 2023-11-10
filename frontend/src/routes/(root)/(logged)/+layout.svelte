<script lang="ts">
	import { BROWSER } from 'esm-env'

	import {
		AppService,
		FlowService,
		OpenAPI,
		RawAppService,
		ScriptService,
		UserService,
		WorkspaceService
	} from '$lib/gen'
	import { classNames } from '$lib/utils'

	import WorkspaceMenu from '$lib/components/sidebar/WorkspaceMenu.svelte'
	import SidebarContent from '$lib/components/sidebar/SidebarContent.svelte'
	import {
		premiumStore,
		starStore,
		superadmin,
		usageStore,
		userStore,
		workspaceStore
	} from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { afterNavigate, beforeNavigate, goto } from '$app/navigation'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import SuperadminSettings from '$lib/components/SuperadminSettings.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { page } from '$app/stores'
	import FavoriteMenu from '$lib/components/sidebar/FavoriteMenu.svelte'
	import { SUPERADMIN_SETTINGS_HASH, USER_SETTINGS_HASH } from '$lib/components/sidebar/settings'
	import { isCloudHosted } from '$lib/cloud'
	import { syncTutorialsTodos } from '$lib/tutorialUtils'
	import { ArrowLeft } from 'lucide-svelte'
	import { getUserExt } from '$lib/user'

	OpenAPI.WITH_CREDENTIALS = true
	let menuOpen = false
	let isCollapsed = false
	let userSettings: UserSettings
	let superadminSettings: SuperadminSettings

	if ($page.status == 404) {
		goto('/user/login')
	}

	$: {
		let queryWorkspace = $page.url.searchParams.get('workspace')
		if (queryWorkspace) {
			$workspaceStore = queryWorkspace
		}
	}

	$: if (userSettings && $page.url.hash === USER_SETTINGS_HASH) {
		userSettings.openDrawer()
	} else if (superadminSettings && $page.url.hash === SUPERADMIN_SETTINGS_HASH) {
		superadminSettings.openDrawer()
	}

	$: updateUserStore($workspaceStore)

	async function updateUserStore(workspace: string | undefined) {
		if (workspace) {
			try {
				localStorage.setItem('workspace', String(workspace))
			} catch (e) {
				console.error('Could not persist workspace to local storage', e)
			}
			const user = await getUserExt(workspace)
			userStore.set(user)
			if (isCloudHosted() && user?.is_admin) {
				premiumStore.set(await WorkspaceService.getPremiumInfo({ workspace }))
			}
		} else {
			userStore.set(undefined)
		}
	}

	beforeNavigate(() => {
		menuOpen = false
	})

	let innerWidth = BROWSER ? window.innerWidth : 2000

	let favoriteLinks = [] as {
		label: string
		href: string
		kind: 'app' | 'script' | 'flow' | 'raw_app'
	}[]
	$: $workspaceStore && $starStore && onLoad()

	function onLoad() {
		loadFavorites()
		loadUsage()
		syncTutorialsTodos()
	}

	async function loadUsage() {
		if (isCloudHosted()) {
			$usageStore = await UserService.getUsage()
		}
	}

	async function loadFavorites() {
		const scripts = await ScriptService.listScripts({
			workspace: $workspaceStore ?? '',
			starredOnly: true
		})
		const flows = await FlowService.listFlows({
			workspace: $workspaceStore ?? '',
			starredOnly: true
		})
		const apps = await AppService.listApps({
			workspace: $workspaceStore ?? '',
			starredOnly: true
		})
		const raw_apps = await RawAppService.listRawApps({
			workspace: $workspaceStore ?? '',
			starredOnly: true
		})
		favoriteLinks = [
			...scripts.map((s) => ({
				label: s.summary || s.path,
				href: `/scripts/get/${s.hash}`,
				kind: 'script' as 'script'
			})),
			...flows.map((f) => ({
				label: f.summary || f.path,
				href: `/flows/get/${f.path}`,
				kind: 'flow' as 'flow'
			})),
			...apps.map((f) => ({
				label: f.summary || f.path,
				href: `/apps/get/${f.path}`,
				kind: 'app' as 'app'
			})),
			...raw_apps.map((f) => ({
				label: f.summary || f.path,
				href: `/apps/get_raw/${f.version}/${f.path}`,
				kind: 'raw_app' as 'raw_app'
			}))
		]
	}

	function pathInAppMode(pathname: string | undefined): boolean {
		if (!pathname) return false
		return (
			pathname.startsWith('/apps') ||
			pathname.startsWith('/flows/add') ||
			pathname.startsWith('/flows/edit') ||
			pathname.startsWith('/scripts/add') ||
			pathname.startsWith('/scripts/edit')
		)
	}
	afterNavigate((n) => {
		if (pathInAppMode(n.to?.url.pathname) && innerWidth >= 768) {
			isCollapsed = true
		}
	})

	$: innerWidth && changeCollapsed()

	function changeCollapsed() {
		if (innerWidth < 1248 && innerWidth >= 768) {
			isCollapsed = true
		} else if ((innerWidth >= 1248 || innerWidth < 768) && !pathInAppMode($page.url.pathname)) {
			isCollapsed = false
		}
	}

	let devOnly = $page.url.pathname.startsWith('/scripts/dev')
</script>

<svelte:window bind:innerWidth />

<UserSettings bind:this={userSettings} />
{#if $page.status == 404}
	<CenteredModal title="Page not found, redirecting you to login">
		<div class="w-full">
			<div class="block m-auto w-20">
				<WindmillIcon height="80px" width="80px" spin="fast" />
			</div>
		</div>
	</CenteredModal>
{:else if $userStore}
	{#if $superadmin}
		<SuperadminSettings bind:this={superadminSettings} />
	{/if}
	<div>
		<div
			class={classNames(
				'relative md:hidden',
				menuOpen ? 'z-40' : 'pointer-events-none',
				devOnly ? 'hidden' : ''
			)}
			role="dialog"
			aria-modal="true"
		>
			<div
				class={classNames(
					'fixed inset-0 dark:bg-[#1e232e] bg-[#202125] dark:bg-opacity-75 bg-opacity-75 transition-opacity ease-linear duration-300 z-40 !dark',
					menuOpen ? 'opacity-100' : 'opacity-0'
				)}
			/>

			<div class="fixed inset-0 flex z-40">
				<div
					class={classNames(
						'relative flex-1 flex flex-col max-w-min w-full bg-surface transition ease-in-out duration-300 transform',
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
							class="ml-1 flex items-center justify-center h-8 w-8 rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white border border-white"
						>
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
					<div class="dark:bg-[#1e232e] bg-[#202125] h-full !dark">
						<div
							class="flex gap-x-2 flex-shrink-0 p-4 font-semibold text-gray-200 w-10"
							class:w-40={!isCollapsed}
						>
							<WindmillIcon white={true} height="20px" width="20px" />
							{#if !isCollapsed}Windmill{/if}
						</div>

						<div class="px-2 py-4 space-y-2 border-y border-gray-500">
							<WorkspaceMenu />
							<FavoriteMenu {favoriteLinks} />
						</div>

						<SidebarContent {isCollapsed} />
					</div>
				</div>
			</div>
		</div>

		<div
			class={classNames(
				'hidden md:flex md:flex-col md:fixed md:inset-y-0 transition-all ease-in-out duration-200 shadow-md z-40 ',
				isCollapsed ? 'md:w-12' : 'md:w-40',
				devOnly ? '!hidden' : ''
			)}
		>
			<div
				class="flex-1 flex flex-col min-h-0 h-screen shadow-lg dark:bg-[#1e232e] bg-[#202125] !dark"
			>
				<button
					on:click={() => {
						goto('/')
					}}
				>
					<div
						class="flex-row flex-shrink-0 px-3.5 py-3.5 font-semibold text-gray-100 text-opacity-70 h-12 flex items-center gap-1.5"
						class:w-40={!isCollapsed}
					>
						<div class:mr-1={!isCollapsed}>
							<WindmillIcon white={true} height="20px" width="20px" />
						</div>
						{#if !isCollapsed}
							<div class="text-sm mt-0.5"> Windmill </div>
						{/if}
					</div>
				</button>
				<div class="px-2 py-4 space-y-2 border-y border-gray-700">
					<WorkspaceMenu {isCollapsed} />
					<FavoriteMenu {favoriteLinks} {isCollapsed} />
				</div>

				<SidebarContent {isCollapsed} />

				<div class="flex-shrink-0 flex px-4 pb-3.5">
					<button
						on:click={() => {
							isCollapsed = !isCollapsed
						}}
					>
						<ArrowLeft
							size={16}
							class={classNames(
								'flex-shrink-0 h-4 w-4 transition-all ease-in-out duration-200 text-white',
								isCollapsed ? 'rotate-180' : 'rotate-0'
							)}
						/>
					</button>
				</div>
			</div>
		</div>
		<div
			class={classNames(
				'w-full flex flex-col flex-1 h-full',
				devOnly ? '!pl-0' : isCollapsed ? 'md:pl-12' : 'md:pl-40',
				'transition-all ease-in-out duration-200'
			)}
		>
			<main class="min-h-screen">
				<div class="relative w-full h-full">
					<div
						class={classNames(
							'py-2 px-2 sm:px-4 md:px-8 flex justify-between items-center shadow-sm max-w-7xl mx-auto md:hidden',
							devOnly ? 'hidden' : ''
						)}
					>
						<button
							type="button"
							on:click={() => {
								menuOpen = true
							}}
							class="h-8 w-8 inline-flex items-center justify-center rounded-md text-tertiary hover:text-primary focus:outline-none focus:ring-2 focus:ring-inset focus:ring-indigo-500"
						>
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
					<slot />
				</div>
			</main>
		</div>
	</div>
{:else}
	<CenteredModal title="Loading user...">
		<div class="w-full">
			<div class="block m-auto w-16">
				<WindmillIcon height="60px" width="60px" spin="fast" />
			</div>
		</div>
	</CenteredModal>
{/if}
