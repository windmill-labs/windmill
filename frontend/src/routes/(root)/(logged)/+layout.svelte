<script lang="ts">
	import { BROWSER } from 'esm-env'

	import {
		AppService,
		AssetService,
		FlowService,
		OpenAPI,
		ScriptService,
		SettingService,
		UserService,
		WorkspaceService
	} from '$lib/gen'
	import {
		capitalize,
		classNames,
		getModifierKey,
		parseDbInputFromAssetSyntax,
		sendUserToast
	} from '$lib/utils'
	import WorkspaceMenu from '$lib/components/sidebar/WorkspaceMenu.svelte'
	import SidebarContent from '$lib/components/sidebar/SidebarContent.svelte'
	import CriticalAlertModal from '$lib/components/sidebar/CriticalAlertModal.svelte'
	import {
		enterpriseLicense,
		isPremiumStore,
		superadmin,
		usageStore,
		workspaceUsageStore,
		userStore,
		workspaceStore,
		type UserExt,
		defaultScripts,
		hubBaseUrlStore,
		usedTriggerKinds,
		devopsRole,
		whitelabelNameStore,
		globalDbManagerDrawer
	} from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { afterNavigate, beforeNavigate } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import SuperadminSettings from '$lib/components/SuperadminSettings.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { page } from '$app/stores'
	import FavoriteMenu, {
		favoriteManager,
		getFavoriteHref,
		getFavoriteLabel
	} from '$lib/components/sidebar/FavoriteMenu.svelte'
	import { SUPERADMIN_SETTINGS_HASH, USER_SETTINGS_HASH } from '$lib/components/sidebar/settings'
	import { isCloudHosted } from '$lib/cloud'
	import { syncTutorialsTodos } from '$lib/tutorialUtils'
	import { ArrowLeft, Search, WandSparkles } from 'lucide-svelte'
	import { getUserExt } from '$lib/user'
	import { twMerge } from 'tailwind-merge'
	import OperatorMenu from '$lib/components/sidebar/OperatorMenu.svelte'
	import GlobalSearchModal from '$lib/components/search/GlobalSearchModal.svelte'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import { loadProtectionRules } from '$lib/workspaceProtectionRules.svelte'
	import { setContext, untrack } from 'svelte'
	import { base } from '$app/paths'
	import { Menubar } from '$lib/components/meltComponents'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import AiChatLayout from '$lib/components/copilot/chat/AiChatLayout.svelte'
	import { DEFAULT_HUB_BASE_URL } from '$lib/hub'
	import DBManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import { watchOnce } from 'runed'
	interface Props {
		children?: import('svelte').Snippet
	}

	const remoteUrlParam = $page.url.searchParams.get('remote_url')
	if (remoteUrlParam) {
		document.cookie = `remote_url=${remoteUrlParam}; path=/; secure; samesite=strict`
		$page.url.searchParams.delete('remote_url')
	}

	let { children }: Props = $props()
	OpenAPI.WITH_CREDENTIALS = true
	let menuOpen = $state(false)
	let globalSearchModal: GlobalSearchModal | undefined = $state(undefined)
	let isCollapsed = $state(false)
	let userSettings: UserSettings | undefined = $state()
	let superadminSettings: SuperadminSettings | undefined = $state()
	let menuHidden = $state(false)

	if ($page.status == 404) {
		goto('/user/login')
	}

	function onQueryChangeUserSettings() {
		if (userSettings && $page.url.hash.startsWith(USER_SETTINGS_HASH)) {
			const mcpMode = $page.url.hash.includes('-mcp')
			userSettings.openDrawer(mcpMode)
		}
	}

	function onQueryChangeAdminSettings() {
		if (superadminSettings && $page.url.hash === SUPERADMIN_SETTINGS_HASH) {
			superadminSettings.openDrawer()
		}
	}

	function onQueryChange() {
		let queryWorkspace = $page.url.searchParams.get('workspace')
		if (queryWorkspace) {
			$workspaceStore = queryWorkspace
		}

		menuHidden =
			$page.url.searchParams.get('nomenubar') === 'true' ||
			$page.url.pathname.startsWith('/oauth/callback/')
	}

	async function updateUserStore(workspace: string | undefined) {
		if (workspace) {
			try {
				sessionStorage.setItem('workspace', String(workspace))
				localStorage.setItem('workspace', String(workspace))
			} catch (e) {
				console.error('Could not persist workspace to local storage', e)
			}
			const user = await getUserExt(workspace)
			userStore.set(user)
			if (isCloudHosted() && user?.is_admin) {
				isPremiumStore.set(await WorkspaceService.getIsPremium({ workspace }))
			}
		} else {
			userStore.set(undefined)
		}
	}

	beforeNavigate((navigation) => {
		menuOpen = false

		// Force page reload when navigating to /apps_raw/add or /apps_raw/edit
		// This ensures the cross-origin isolation headers are fetched from the server
		// which are required for SharedArrayBuffer and TypeScript workers to work correctly
		const toPath = navigation.to?.url.pathname
		if (toPath && (toPath.startsWith('/apps_raw/add') || toPath.startsWith('/apps_raw/edit'))) {
			const currentPath = navigation.from?.url.pathname
			// Reload if we're not on an apps_raw path, or if we're on /apps/get_raw/ (viewing a raw app)
			// The /apps/get_raw/ path doesn't have cross-origin isolation headers, so we need to reload
			if (!currentPath?.startsWith('/apps_raw/') || currentPath?.startsWith('/apps_raw/get/')) {
				navigation.cancel()
				window.location.href = navigation.to!.url.href
			}
		}
	})

	let innerWidth = $state(BROWSER ? window.innerWidth : 2000)

	function onLoad() {
		loadFavorites()
		loadUsage()
		syncTutorialsTodos()
		loadHubBaseUrl()
		loadUsedTriggerKinds()
	}

	async function loadUsage() {
		if (isCloudHosted() && $workspaceStore) {
			$usageStore = await UserService.getUsage()
			$workspaceUsageStore = await WorkspaceService.getWorkspaceUsage({
				workspace: $workspaceStore!
			})
		}
	}

	async function loadHubBaseUrl() {
		$hubBaseUrlStore =
			((await SettingService.getGlobal({ key: 'hub_accessible_url' })) as string) ||
			((await SettingService.getGlobal({ key: 'hub_base_url' })) as string) ||
			DEFAULT_HUB_BASE_URL
	}

	async function loadFavorites() {
		const scripts = await ScriptService.listScripts({
			workspace: $workspaceStore ?? '',
			starredOnly: true,
			includeWithoutMain: true,
			withoutDescription: true
		})
		const flows = await FlowService.listFlows({
			workspace: $workspaceStore ?? '',
			starredOnly: true,
			withoutDescription: true
		})
		const apps = await AppService.listApps({
			workspace: $workspaceStore ?? '',
			starredOnly: true
		})
		const assets = await AssetService.listFavoriteAssets({ workspace: $workspaceStore ?? '' })
		favoriteManager.current = [
			...scripts.map((s) => ({
				label: s.summary || getFavoriteLabel(s.path, 'script'),
				path: s.path,
				href: getFavoriteHref(s.path, 'script'),
				kind: 'script' as const
			})),
			...flows.map((f) => ({
				label: f.summary || getFavoriteLabel(f.path, 'flow'),
				path: f.path,
				href: getFavoriteHref(f.path, 'flow'),
				kind: 'flow' as const
			})),
			...apps.map((f) => ({
				label: f.summary || getFavoriteLabel(f.path, 'app'),
				path: f.path,
				href: getFavoriteHref(f.path, 'app'),
				kind: 'app' as const
			})),
			...assets.map((a) => ({
				label: getFavoriteLabel(a.path, 'asset'),
				path: a.path,
				href: getFavoriteHref(a.path, 'asset'),
				kind: 'asset' as const
			}))
		]
	}

	async function loadUsedTriggerKinds() {
		let usedKinds: string[] = []
		const {
			http_routes_used,
			websocket_used,
			kafka_used,
			postgres_used,
			nats_used,
			sqs_used,
			mqtt_used,
			gcp_used,
			email_used,
			nextcloud_used,
			google_used
		} = await WorkspaceService.getUsedTriggers({
			workspace: $workspaceStore ?? ''
		})
		if (http_routes_used) {
			usedKinds.push('http')
		}
		if (websocket_used) {
			usedKinds.push('ws')
		}
		if (kafka_used) {
			usedKinds.push('kafka')
		}
		if (postgres_used) {
			usedKinds.push('postgres')
		}
		if (nats_used) {
			usedKinds.push('nats')
		}
		if (mqtt_used) {
			usedKinds.push('mqtt')
		}
		if (sqs_used) {
			usedKinds.push('sqs')
		}
		if (gcp_used) {
			usedKinds.push('gcp')
		}
		if (email_used) {
			usedKinds.push('email')
		}
		if (nextcloud_used) {
			usedKinds.push('nextcloud')
		}
		if (google_used) {
			usedKinds.push('google')
		}
		$usedTriggerKinds = usedKinds
	}

	function pathInAppMode(pathname: string | undefined): boolean {
		if (!pathname) return false
		return (
			pathname.startsWith(base + '/apps') ||
			pathname.startsWith(base + '/flows/add') ||
			pathname.startsWith(base + '/flows/edit') ||
			pathname.startsWith(base + '/scripts/add') ||
			pathname.startsWith(base + '/scripts/edit')
		)
	}
	afterNavigate((n) => {
		if (pathInAppMode(n.to?.url.pathname) && innerWidth >= 768) {
			isCollapsed = true
		}
	})

	function changeCollapsed() {
		if (innerWidth < 1248 && innerWidth >= 768 && !isCollapsed) {
			isCollapsed = true
		}
	}

	let devOnly = $derived($page.url.pathname.startsWith(base + '/scripts/dev'))

	async function loadDefaultScripts(workspace: string, user: UserExt | undefined) {
		if (!user?.operator) {
			$defaultScripts = await WorkspaceService.getDefaultScripts({ workspace })
		}
	}
	let timeout: number | undefined
	async function onUserStore(u: UserExt | undefined) {
		if (u && timeout) {
			clearTimeout(timeout)
			timeout = undefined
		} else if (!u) {
			timeout = setTimeout(async () => {
				if (!$userStore && $workspaceStore) {
					$userStore = await getUserExt($workspaceStore)
				}
			}, 5000)
		}
	}

	function openSearchModal(text?: string): void {
		globalSearchModal?.openSearchWithPrefilledText(text)
	}

	setContext('openSearchWithPrefilledText', openSearchModal)

	let numUnacknowledgedCriticalAlerts = $state(0)
	let mountModal = $state(false)
	let isCriticalAlertsUiMuted = $state(true)
	let muteSettings = $state({
		global: true,
		workspace: true
	})
	async function loadCriticalAlertsMuted() {
		let g_muted = true
		const ws_muted =
			(await WorkspaceService.getSettings({ workspace: $workspaceStore! })).mute_critical_alerts ||
			false

		if ($superadmin) {
			g_muted = (await SettingService.getGlobal({
				key: 'critical_alert_mute_ui'
			})) as boolean
			isCriticalAlertsUiMuted = g_muted
		} else {
			isCriticalAlertsUiMuted = ws_muted
		}

		muteSettings = { global: g_muted, workspace: ws_muted }
	}

	async function checkTeamPlanStatus(workspace: string) {
		const premiumInfo = await WorkspaceService.getPremiumInfo({
			workspace,
			skipSubscriptionFetch: true // won't load subscription status from stripe but only the past due status from db
		})
		if (premiumInfo.is_past_due) {
			if (
				premiumInfo.max_tolerated_executions === undefined ||
				(premiumInfo.usage ?? 0) > premiumInfo.max_tolerated_executions
			) {
				sendUserToast(
					'Your last invoice is unpaid, you cannot run any more jobs. Please update your payment method in the workspace settings to continue running jobs.',
					true
				)
			} else {
				sendUserToast(
					'Your last invoice is unpaid. Please update your payment method in the workspace settings to prevent the interruption of your job executions.',
					true
				)
			}
		}
	}

	$effect(() => {
		$page.url && userSettings != undefined && untrack(() => onQueryChangeUserSettings())
	})
	$effect(() => {
		$page.url && superadminSettings != undefined && untrack(() => onQueryChangeAdminSettings())
	})
	$effect(() => {
		$page.url && untrack(() => onQueryChange())
	})
	$effect(() => {
		$workspaceStore
		untrack(() => updateUserStore($workspaceStore))
	})
	$effect(() => {
		$workspaceStore && untrack(() => onLoad())
	})
	$effect(() => {
		innerWidth && untrack(() => changeCollapsed())
	})
	$effect(() => {
		$userStore
		untrack(() => onUserStore($userStore))
	})
	$effect(() => {
		$workspaceStore && $userStore && untrack(() => loadDefaultScripts($workspaceStore!, $userStore))
	})
	$effect(() => {
		if (isCollapsed && $userStore?.operator) {
			isCollapsed = false
		}
	})
	$effect(() => {
		if (
			$enterpriseLicense &&
			$workspaceStore &&
			$userStore &&
			$devopsRole !== undefined &&
			($devopsRole || $userStore.is_admin)
		) {
			mountModal = true
			untrack(() => loadCriticalAlertsMuted())
		} else {
			mountModal = false
		}
	})

	$effect(() => {
		if (isCloudHosted()) {
			const workspace = $workspaceStore
			if (workspace && $userStore?.is_admin) {
				checkTeamPlanStatus(workspace)
			}
		}
	})

	// Load workspace protection rules on workspace change
	$effect(() => {
		const workspace = $workspaceStore
		if (workspace) {
			untrack(() => loadProtectionRules(workspace))
		}
	})
	watchOnce(
		() => globalDbManagerDrawer.val,
		() => {
			if (!globalDbManagerDrawer.val) return
			const hash = window.location.hash
			if (hash.startsWith('#dbmanager:')) {
				const [_, path] = hash.split('#dbmanager:')
				const dbInput = parseDbInputFromAssetSyntax(path)
				if (dbInput) globalDbManagerDrawer.val?.openDrawer(dbInput)
			}
		}
	)
</script>

<svelte:window bind:innerWidth />

<UserSettings bind:this={userSettings} showMcpMode={true} />
{#if $page.status == 404}
	<CenteredModal title="Page not found, redirecting you to login" loading={true}></CenteredModal>
{:else if $userStore}
	<GlobalSearchModal bind:this={globalSearchModal} />
	{#if $superadmin}
		<SuperadminSettings bind:this={superadminSettings} />
	{/if}
	{#if mountModal}
		<CriticalAlertModal bind:muteSettings bind:numUnacknowledgedCriticalAlerts />
	{/if}
	<div class="h-screen flex flex-col">
		{#if !menuHidden}
			{#if !$userStore?.operator}
				{#if innerWidth < 768}
					<div
						class={classNames(
							'relative',
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
						></div>

						<div class="fixed inset-0 flex z-40">
							<div
								class={classNames(
									'relative flex-1 flex flex-col max-w-min w-full bg-surface transition ease-in-out duration-300 transform',
									menuOpen ? 'translate-x-0' : '-translate-x-full'
								)}
							>
								<div
									class={classNames(
										'absolute top-0 right-4 -mr-12 pt-2 ease-in-out duration-300',
										menuOpen ? 'opacity-100' : 'opacity-0'
									)}
								>
									<button
										type="button"
										onclick={() => {
											menuOpen = !menuOpen
										}}
										class="ml-1 flex items-center justify-center h-6 w-6 rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white border border-white"
										aria-label="Close"
									>
										<svg
											class="h-4 w-4 text-white"
											xmlns="http://www.w3.org/2000/svg"
											fill="none"
											viewBox="0 0 24 24"
											stroke-width="2"
											stroke="currentColor"
											aria-hidden="true"
										>
											<path
												stroke-linecap="round"
												stroke-linejoin="round"
												d="M6 18L18 6M6 6l12 12"
											/>
										</svg>
									</button>
								</div>
								<div class="dark:bg-[#1e232e] bg-[#202125] h-full !dark flex flex-col">
									<div class="flex gap-x-2 flex-shrink-0 p-4 font-semibold text-gray-200 w-40">
										<WindmillIcon white={true} height="20px" width="20px" />
										{#if $whitelabelNameStore}
											{$whitelabelNameStore}
										{:else}
											Windmill
										{/if}
									</div>
									<div class="px-2 py-4 border-y border-gray-500">
										<Menubar>
											{#snippet children({ createMenu })}
												<WorkspaceMenu {createMenu} />
												<FavoriteMenu {createMenu} favoriteLinks={favoriteManager.current} />
											{/snippet}
										</Menubar>
										<MenuButton
											stopPropagationOnClick={true}
											on:click={() => openSearchModal()}
											isCollapsed={false}
											icon={Search}
											label="Search"
											class="!text-xs"
											shortcut={`${getModifierKey()}k`}
										/>
										<MenuButton
											stopPropagationOnClick={true}
											on:click={() => aiChatManager.toggleOpen()}
											isCollapsed={false}
											icon={WandSparkles}
											iconProps={{
												forceDarkMode: true
											}}
											label="Ask AI"
											class="!text-xs"
											iconClasses="!text-ai-inverse dark:!text-ai"
											shortcut={`${getModifierKey()}L`}
										/>
									</div>

									<SidebarContent
										isCollapsed={false}
										numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
											? 0
											: numUnacknowledgedCriticalAlerts}
									/>
								</div>
							</div>
						</div>
					</div>
				{:else}
					<div
						id="sidebar"
						class={classNames(
							'flex flex-col fixed inset-y-0 transition-all ease-in-out duration-200 shadow-md z-40 ',
							isCollapsed ? 'md:w-12' : 'md:w-40',
							devOnly ? '!hidden' : ''
						)}
					>
						<div
							class="flex-1 flex flex-col min-h-0 h-screen shadow-lg dark:bg-[#1e232e] bg-[#202125] !dark"
						>
							<button
								onclick={() => {
									goto('/')
								}}
							>
								<div
									class="flex-row flex-shrink-0 px-3.5 py-3.5 text-opacity-70 h-12 flex items-center gap-1.5"
									class:w-40={!isCollapsed}
								>
									<div class:mr-1={!isCollapsed}>
										<WindmillIcon white={true} height="20px" width="20px" />
									</div>
									{#if !isCollapsed}
										<div class="text-sm mt-0.5 text-white">
											{#if $whitelabelNameStore}{capitalize(
													$whitelabelNameStore
												)}{:else}Windmill{/if}
										</div>
									{/if}
								</div>
							</button>
							<div class="px-2 py-4 border-y border-gray-700 flex flex-col gap-1">
								<Menubar class="flex flex-col gap-1">
									{#snippet children({ createMenu })}
										<WorkspaceMenu {createMenu} {isCollapsed} />
										<FavoriteMenu
											{createMenu}
											favoriteLinks={favoriteManager.current}
											{isCollapsed}
										/>
									{/snippet}
								</Menubar>
								<MenuButton
									stopPropagationOnClick={true}
									on:click={() => openSearchModal()}
									{isCollapsed}
									icon={Search}
									label="Search"
									class="!text-xs"
									shortcut={`${getModifierKey()}k`}
								/>
								<MenuButton
									stopPropagationOnClick={true}
									on:click={() => aiChatManager.toggleOpen()}
									{isCollapsed}
									icon={WandSparkles}
									iconProps={{
										forceDarkMode: true
									}}
									label="Ask AI"
									class="!text-xs"
									iconClasses="!text-ai-inverse dark:!text-ai"
									shortcut={`${getModifierKey()}L`}
								/>
							</div>

							<SidebarContent
								{isCollapsed}
								numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
									? 0
									: numUnacknowledgedCriticalAlerts}
							/>

							<div class="flex-shrink-0 flex px-4 pb-3.5">
								<button
									onclick={() => {
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
				{/if}
			{:else}
				<div class="absolute top-2 left-2 z5000">
					<OperatorMenu favoriteLinks={favoriteManager.current} />
				</div>
			{/if}
			<!-- Legacy menu -->
			<div
				class={classNames(
					'fixed inset-0 dark:bg-[#1e232e] bg-[#202125] dark:bg-opacity-75 bg-opacity-75 transition-opacity ease-linear duration-300  !dark',
					'opacity-0 pointer-events-none'
				)}
			>
				<div class={twMerge('fixed inset-0 flex ', '-z-0')}>
					<div
						class={classNames(
							'relative flex-1 flex flex-col max-w-min w-full bg-surface transition ease-in-out duration-100 transform',
							'-translate-x-full'
						)}
					>
						<div
							class={classNames(
								'absolute top-0 right-0 -mr-12 pt-2 ease-in-out duration-100',
								'opacity-0'
							)}
						>
							<button
								type="button"
								onclick={() => {
									// menuSlide = !menuSlide
								}}
								aria-label="Close"
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
								{#if !isCollapsed}{#if $whitelabelNameStore}{capitalize(
											$whitelabelNameStore
										)}{:else}Windmill{/if}{/if}
							</div>

							<div class="px-2 py-4 space-y-2 border-y border-gray-500">
								<Menubar>
									{#snippet children({ createMenu })}
										<WorkspaceMenu {createMenu} />
										<FavoriteMenu {createMenu} favoriteLinks={favoriteManager.current} />
									{/snippet}
								</Menubar>
								<MenuButton
									stopPropagationOnClick={true}
									on:click={() => openSearchModal()}
									{isCollapsed}
									icon={Search}
									label="Search"
									class="!text-xs"
									shortcut={`${getModifierKey()}k`}
								/>
								<MenuButton
									stopPropagationOnClick={true}
									on:click={() => aiChatManager.toggleOpen()}
									{isCollapsed}
									icon={WandSparkles}
									iconProps={{
										forceDarkMode: true
									}}
									label="Ask AI"
									class="!text-xs"
									iconClasses="!text-ai-inverse dark:!text-ai"
									shortcut={`${getModifierKey()}L`}
								/>
							</div>

							<SidebarContent
								{isCollapsed}
								numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
									? 0
									: numUnacknowledgedCriticalAlerts}
							/>
						</div>
					</div>
				</div>
			</div>
		{/if}
		<div class="flex flex-col h-full w-full">
			<AiChatLayout
				{children}
				noPadding={devOnly}
				{isCollapsed}
				onMenuOpen={() => {
					menuOpen = true
				}}
			/>
		</div>
	</div>
{:else}
	<CenteredModal title="Loading user..." loading={true}></CenteredModal>
{/if}

{#if $workspaceStore}
	<DBManagerDrawer bind:this={globalDbManagerDrawer.val} />
{/if}
