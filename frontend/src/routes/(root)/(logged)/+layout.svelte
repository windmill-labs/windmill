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
	import { capitalize, classNames, getModifierKey, sendUserToast } from '$lib/utils'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import WorkspaceMenu from '$lib/components/sidebar/WorkspaceMenu.svelte'
	import SidebarContent from '$lib/components/sidebar/SidebarContent.svelte'
	import SettingsMenu from '$lib/components/sidebar/SettingsMenu.svelte'
	import CriticalAlertModal from '$lib/components/sidebar/CriticalAlertModal.svelte'
	import ForkConflictModal from '$lib/components/ForkConflictModal.svelte'
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
		wsBaseUrlStore,
		disableHubStore,
		usedTriggerKinds,
		devopsRole,
		whitelabelNameStore,
		globalDbManagerDrawer,
		globalForkModal
	} from '$lib/stores'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import { afterNavigate, beforeNavigate } from '$app/navigation'
	import { goto } from '$lib/navigation'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import SuperadminSettings from '$lib/components/SuperadminSettings.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import { page } from '$app/state'
	import FavoriteMenu, {
		favoriteManager,
		getFavoriteHref,
		getFavoriteLabel
	} from '$lib/components/sidebar/FavoriteMenu.svelte'
	import { SUPERADMIN_SETTINGS_HASH, USER_SETTINGS_HASH } from '$lib/components/sidebar/settings'
	import { isCloudHosted } from '$lib/cloud'
	import { syncTutorialsTodos } from '$lib/tutorialUtils'
	import { ArrowLeft, Home, Play, Search, WandSparkles } from 'lucide-svelte'
	import { getUserExt } from '$lib/user'
	import { deepEqual } from 'fast-equals'
	import { twMerge } from 'tailwind-merge'
	import OperatorMenu from '$lib/components/sidebar/OperatorMenu.svelte'
	import GlobalSearchModal from '$lib/components/search/GlobalSearchModal.svelte'
	import MenuButton from '$lib/components/sidebar/MenuButton.svelte'
	import MenuLink from '$lib/components/sidebar/MenuLink.svelte'
	import { loadProtectionRules } from '$lib/workspaceProtectionRules.svelte'
	import { purgeLegacyUserDrafts } from '$lib/userDraftLegacyMigration'
	import { migrateUserDraftsToDb } from '$lib/userDraftDbMigration'
	import DraftMigrationErrorModal from '$lib/components/DraftMigrationErrorModal.svelte'
	import { setContext, untrack } from 'svelte'
	import { base } from '$app/paths'
	import { Menubar } from '$lib/components/meltComponents'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import AiChatLayout from '$lib/components/copilot/chat/AiChatLayout.svelte'
	import SessionPicker from '$lib/components/sessions/SessionPicker.svelte'
	import SessionModeSwitch from '$lib/components/sessions/SessionModeSwitch.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { parsePreviewItemRoute } from '$lib/components/sessions/previewRouter'
	import { rememberNavRoute } from '$lib/components/sessions/sessionSwitch.svelte'
	import WorkspaceScopeHeader from '$lib/components/sidebar/WorkspaceScopeHeader.svelte'
	import { DEFAULT_HUB_BASE_URL } from '$lib/hub'
	import DBManagerDrawer from '$lib/components/DBManagerDrawer.svelte'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import { useDbManagerUriState } from '$lib/components/dbManagerDrawerModel.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import CreateWorkspaceInner from '$lib/components/workspaceSettings/CreateWorkspaceInner.svelte'
	interface Props {
		children?: import('svelte').Snippet
	}

	const remoteUrlParam = page.url.searchParams.get('remote_url')
	if (remoteUrlParam) {
		document.cookie = `remote_url=${remoteUrlParam}; path=/; secure; samesite=strict`
		page.url.searchParams.delete('remote_url')
	}

	let { children }: Props = $props()
	OpenAPI.WITH_CREDENTIALS = true
	let menuOpen = $state(false)
	// Set by the workspace⇄session switch before it navigates, so the mobile menu
	// drawer stays open across a mode toggle (unlike a normal link navigation,
	// which dismisses it). Consumed once in beforeNavigate.
	let preserveMenuOnNextNav = false
	let globalSearchModal: GlobalSearchModal | undefined = $state(undefined)
	// Persisted nav-rail collapse preference. Only the manual toggle writes to it;
	// the contextual auto-collapse (app-mode routes, narrow widths) mutates the
	// in-memory `isCollapsed` without persisting, so it stays transient and never
	// gets "stuck" collapsed across reloads.
	const collapsePref = useLocalStorageValue<boolean>('nav_menu_collapsed', false, 'boolean')
	let isCollapsed = $state(collapsePref.val)
	let userSettings: UserSettings | undefined = $state()
	let superadminSettings: SuperadminSettings | undefined = $state()
	let menuHidden = $state(false)
	let isDarkMode = useIsDarkMode()
	let darkMode = $derived(isDarkMode.val)

	// Session mode is route-derived: the rail shows the sessions sidebar on the
	// /sessions page and the workspace navigation everywhere else. The switch
	// (SessionModeSwitch) just navigates in and out of that route.
	let sessionMode = $derived(page.url.pathname.startsWith(base + '/sessions'))
	// Inside a preview iframe the rail still renders (navigation mode), but the
	// switch must not — entering session mode from within the preview would
	// nest the whole experience. Hide it when embedded.
	const embedded = BROWSER && window.self !== window.top

	// AI sessions are still dev-gated (localStorage wm_dev_global_ai=1), same as
	// the global chat. The Workspace ⇄ Sessions switch is the only entry point, so
	// gate it on the flag too — otherwise it would ship the unfinished experience
	// to prod. The /sessions page has its own gate for direct navigation.
	const globalAiEnabled = isGlobalAiEnabled()

	const SIDEBAR_BG = '#F3F3F7'
	const SIDEBAR_BG_DARK = '#1e232e'

	if (page.status == 404) {
		goto('/user/login')
	}

	function onQueryChangeUserSettings() {
		if (userSettings && page.url.hash.startsWith(USER_SETTINGS_HASH)) {
			const mcpMode = page.url.hash.includes('-mcp')
			userSettings.openDrawer(mcpMode)
		}
	}

	function onQueryChangeAdminSettings() {
		if (superadminSettings && page.url.hash === SUPERADMIN_SETTINGS_HASH) {
			superadminSettings.openDrawer()
		}
	}

	function onQueryChange() {
		let queryWorkspace = page.url.searchParams.get('workspace')
		if (queryWorkspace) {
			$workspaceStore = queryWorkspace
		}

		// When this window is an iframe (e.g. the sessions preview), keep the menu
		// hidden once `nomenubar` has been requested: navigating inside the preview
		// drops the query param (both client-side routing and full-document loads),
		// and we don't want the global nav to pop back in. Stickiness is stored in
		// sessionStorage so it survives full reloads within the iframe's browsing
		// context. The top window is unaffected (embedded is false there), so the
		// oauth-callback case and ordinary navigation still toggle normally.
		const embedded = typeof window !== 'undefined' && window.self !== window.top
		const requested = page.url.searchParams.get('nomenubar') === 'true'
		if (embedded && requested) {
			try {
				sessionStorage.setItem('nomenubar_embedded', 'true')
			} catch {}
		}
		let stickyEmbedded = false
		if (embedded) {
			try {
				stickyEmbedded = sessionStorage.getItem('nomenubar_embedded') === 'true'
			} catch {}
		}
		menuHidden = requested || page.url.pathname.startsWith('/oauth/callback/') || stickyEmbedded
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
			if (!deepEqual(user, $userStore)) {
				userStore.set(user)
			}
			// Persist the workspace username so the synchronous `/add` →
			// `/edit/u/{user}/draft_{uuid}` redirects (in each editor's
			// `+page.ts`) can land on the right namespace without waiting
			// for this async layout fetch. Without this they fall back to
			// the `'me'` placeholder on every fresh nav.
			try {
				if (user?.username) localStorage.setItem('username', user.username)
			} catch (e) {
				console.error('Could not persist username to local storage', e)
			}
			if (isCloudHosted() && user?.is_admin) {
				isPremiumStore.set(await WorkspaceService.getIsPremium({ workspace }))
			}
		} else {
			userStore.set(undefined)
		}
	}

	// True when this window is a sessions-preview iframe (embedded + nomenubar,
	// which the preview always sets and stickies — see the menu-hide block above).
	function isSessionPreviewEmbed(): boolean {
		if (!embedded) return false
		try {
			return sessionStorage.getItem('nomenubar_embedded') === 'true'
		} catch {
			return false
		}
	}

	// Map a navigation target to the session editor it should open as a component,
	// or undefined for anything without a live-editor wrapper (regular apps, pages,
	// /get viewers). Only /edit routes — the wrappers are editors.
	function previewEditorTarget(
		url: URL | undefined
	): { kind: 'script' | 'flow' | 'raw_app'; path: string } | undefined {
		if (!url || !/\/(scripts|flows|apps_raw)\/edit\//.test(url.pathname)) return undefined
		const route = parsePreviewItemRoute(url.pathname)
		if (!route) return undefined
		const kind =
			route.kind === 'script'
				? 'script'
				: route.kind === 'flow'
					? 'flow'
					: route.raw_app
						? 'raw_app'
						: undefined
		return kind ? { kind, path: route.itemPath } : undefined
	}

	beforeNavigate((navigation) => {
		if (preserveMenuOnNextNav) {
			preserveMenuOnNextNav = false
		} else {
			menuOpen = false
		}

		// Inside a sessions-preview iframe, hand an editor-route navigation up to the
		// parent so it mounts the in-process editor (sharing the session runtime)
		// instead of booting a second, disconnected editor in this frame. Cancel so
		// the heavy editor never mounts here at all. Runs before the apps_raw reload
		// below so a raw-app editor promotes rather than full-reloading the iframe.
		if (isSessionPreviewEmbed()) {
			const target = previewEditorTarget(navigation.to?.url)
			if (target) {
				navigation.cancel()
				try {
					window.parent.postMessage(
						{ type: 'wm.session.openEditor', kind: target.kind, path: target.path },
						window.location.origin
					)
				} catch {}
				return
			}
		}

		// Force page reload when navigating to /apps_raw/add or /apps_raw/edit
		// This ensures the cross-origin isolation headers are fetched from the server
		// which are required for SharedArrayBuffer and TypeScript workers to work correctly
		const toPath = navigation.to?.url.pathname
		if (toPath && (toPath.startsWith('/apps_raw/add') || toPath.startsWith('/apps_raw/edit'))) {
			const currentPath = navigation.from?.url.pathname
			// Reload if we're not on an apps_raw path, or if we're on the raw app viewer
			// (/apps_raw/get/): the viewer doesn't have cross-origin isolation headers, so
			// we need a full reload to fetch them for the editor.
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
		loadWsBaseUrl()
		loadDisableHub()
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

	async function loadWsBaseUrl() {
		const val = (await SettingService.getGlobal({ key: 'ws_base_url' })) as string
		if (val) {
			$wsBaseUrlStore = val
		}
	}

	async function loadDisableHub() {
		$disableHubStore =
			((await SettingService.getGlobal({ key: 'disable_hub' })) as boolean) ?? false
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
			azure_used,
			email_used,
			nextcloud_used,
			google_used,
			github_used
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
		if (azure_used) {
			usedKinds.push('azure')
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
		if (github_used) {
			usedKinds.push('github')
		}
		$usedTriggerKinds = usedKinds
	}

	afterNavigate((n) => {
		// Remember the last navigation-mode route so exiting session mode returns
		// the user where they were rather than to the home page.
		const to = n.to?.url
		if (to && !to.pathname.startsWith(base + '/sessions')) {
			rememberNavRoute(to.pathname + to.search)
		}
	})

	function changeCollapsed() {
		if (innerWidth < 1248 && innerWidth >= 768 && !isCollapsed) {
			isCollapsed = true
		}
	}

	let devOnly = $derived(page.url.pathname.startsWith(base + '/scripts/dev'))

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
			(await WorkspaceService.getPublicSettings({ workspace: $workspaceStore! }))
				.mute_critical_alerts || false

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
		page.url && userSettings != undefined && untrack(() => onQueryChangeUserSettings())
	})
	$effect(() => {
		page.url && superadminSettings != undefined && untrack(() => onQueryChangeAdminSettings())
	})
	$effect(() => {
		page.url && untrack(() => onQueryChange())
	})
	$effect(() => {
		$workspaceStore
		untrack(() => updateUserStore($workspaceStore))
	})
	$effect(() => {
		$workspaceStore && untrack(() => onLoad())
	})
	// One-shot UserDraft migration. `purgeLegacyUserDrafts` drops the oldest
	// workspace-blind `flow` / `app-…` / `rawapp-…` LS autosave keys (they
	// can't be attributed to a workspace, so promoting them would mis-file
	// drafts). `migrateUserDraftsToDb` then pushes the workspace-scoped
	// `userdraft/w/{ws}/{kind}/{path}` keys — written by the editor with the
	// correct workspace — onto the server-side draft table, clearing LS on
	// success.
	$effect(() => {
		if ($workspaceStore && $userStore) {
			untrack(() => {
				purgeLegacyUserDrafts()
				void migrateUserDraftsToDb()
			})
		}
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

	globalDbManagerDrawer.val = useDbManagerUriState()
</script>

<svelte:window bind:innerWidth />

<!-- Home + Runs lifted to the top of the workspace nav, sitting with Favorites
     and Search as the primary quick-access cluster. Excluded from SidebarContent
     (via excludeMainLabels) so they aren't rendered twice. -->
{#snippet quickLinks(collapsed: boolean)}
	<MenuLink
		class="!text-xs"
		label="Home"
		href={`${base}/`}
		icon={Home}
		isCollapsed={collapsed}
		aiId="sidebar-menu-link-home"
		aiDescription="Button to navigate to home which contains all the user's scripts, flows and apps"
	/>
	<MenuLink
		class="!text-xs"
		label="Runs"
		href={`${base}/runs`}
		icon={Play}
		isCollapsed={collapsed}
		aiId="sidebar-menu-link-runs"
		aiDescription="Button to navigate to runs"
		onclick={() => {
			setTimeout(() => {
				window.dispatchEvent(new Event('popstate'))
			}, 100)
		}}
	/>
{/snippet}

<UserSettings bind:this={userSettings} showMcpMode={true} />
<DraftMigrationErrorModal />
{#if page.status == 404}
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
								'fixed inset-0 bg-black/50 transition-opacity ease-linear duration-300 z-40',

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
								<div
									class="h-full flex flex-col"
									style:background-color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}
								>
									<!-- Workspace picker as the drawer header (replaces the Windmill logo). -->
									<div class="flex-shrink-0 px-2 h-12 w-52 flex items-center">
										<Menubar class="w-full">
											{#snippet children({ createMenu })}
												<WorkspaceMenu {createMenu} />
											{/snippet}
										</Menubar>
									</div>

									{#if !embedded && globalAiEnabled}
										<!-- The switch: workspace navigation ⇄ sessions sidebar. -->
										<div class="px-2 pb-1 w-52">
											<SessionModeSwitch
												mode={sessionMode ? 'session' : 'nav'}
												onToggle={() => (preserveMenuOnNextNav = true)}
											/>
										</div>
									{/if}

									{#if !sessionMode}
										<!-- Workspace scope (fork picker): part of the top workspace group. -->
										<div class="pb-1 w-52 {globalAiEnabled ? '' : '-mt-1'}">
											<WorkspaceScopeHeader isCollapsed={false} />
										</div>
									{/if}

									{#if sessionMode}
										<!-- Session mode: the session list owns the rail.
										     w-52 cap (matches the desktop sidebar width): the drawer is
										     max-w-min, and long session titles (nowrap before truncation)
										     would otherwise inflate its min-content width to the full
										     text width. -->
										<div class="px-2 py-2 w-52">
											<SessionPicker isCollapsed={false} embedded collapsible={false} />
										</div>
									{:else}
										<!-- Navigation mode: the classic workspace navigation. -->
										<!-- Workspace-scoped region: Home/Runs + Favorites + Search + workspace items. -->
										<div class="px-2 pt-1 pb-2 w-52 flex flex-col gap-1">
											{@render quickLinks(false)}
											<Menubar>
												{#snippet children({ createMenu })}
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
											{#if !globalAiEnabled}
												<!-- Global Ask-AI pane. When the sessions dev flag is on it is
												     replaced by SessionModeSwitch, so it only shows in prod. -->
												<MenuButton
													stopPropagationOnClick={true}
													on:click={() => aiChatManager.toggleOpen()}
													isCollapsed={false}
													icon={WandSparkles}
													iconProps={{ forceDarkMode: true }}
													label="Ask AI"
													class="!text-xs"
													iconClasses="!text-ai"
													shortcut={`${getModifierKey()}L`}
												/>
											{/if}
										</div>

										<SidebarContent
											isCollapsed={false}
											showSecondary={false}
											excludeMainLabels={['Home', 'Runs']}
											numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
												? 0
												: numUnacknowledgedCriticalAlerts}
										/>
									{/if}

									<!-- Settings dropdown stays across both modes; session mode just hides
									     the workspace-settings entry. -->
									<div class="px-2 pb-2">
										<SettingsMenu isCollapsed={false} hideWorkspaceSettings={sessionMode} />
									</div>
								</div>
							</div>
						</div>
					</div>
				{:else}
					<div
						id="sidebar"
						class={classNames(
							'flex flex-col fixed inset-y-0 transition-all ease-in-out duration-200 z-40 ',
							isCollapsed ? 'w-12' : 'w-52',
							devOnly ? '!hidden' : ''
						)}
					>
						<div
							class="flex-1 flex flex-col min-h-0 h-screen border-r border-light dark:border-gray-700"
							style:background-color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}
						>
							<!-- Workspace picker as the sidebar header (replaces the Windmill logo).
							     Kept in both modes: it scopes which workspace family's sessions
							     the sessions sidebar shows. -->
							<div class="flex-shrink-0 px-2 h-12 flex items-center">
								<Menubar class="w-full">
									{#snippet children({ createMenu })}
										<WorkspaceMenu {createMenu} {isCollapsed} />
									{/snippet}
								</Menubar>
							</div>

							{#if !embedded && globalAiEnabled}
								<!-- The switch: workspace navigation ⇄ sessions sidebar. -->
								<div class="px-2 pb-1 {isCollapsed ? 'flex justify-center' : ''}">
									<SessionModeSwitch mode={sessionMode ? 'session' : 'nav'} {isCollapsed} />
								</div>
							{/if}

							{#if !sessionMode}
								<!-- Workspace scope (fork picker): part of the top workspace group,
								     together with the family menu and the mode switch above. Without
								     the switch, pull it up so the group still reads as one block. -->
								<div class="pb-1 {globalAiEnabled ? '' : '-mt-1'}">
									<WorkspaceScopeHeader {isCollapsed} />
								</div>
							{/if}

							{#if sessionMode}
								<!-- Session mode: the session list owns the rail. Workspace nav moves
								     into the preview side panel (the iframe renders its own nav). -->
								<div class="px-2 py-2 flex flex-col gap-1 flex-1 min-h-0 overflow-y-auto">
									<SessionPicker {isCollapsed} embedded collapsible={false} />
								</div>
							{:else}
								<!-- Navigation mode: the classic workspace navigation. -->
								<!-- Workspace-scoped region: Home/Runs + Favorites + Search + workspace items. -->
								<div class="px-2 pt-1 pb-2 flex flex-col gap-1">
									{@render quickLinks(isCollapsed)}
									<Menubar class="flex flex-col gap-1">
										{#snippet children({ createMenu })}
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
									{#if !globalAiEnabled}
										<!-- Global Ask-AI pane. When the sessions dev flag is on it is
										     replaced by SessionModeSwitch, so it only shows in prod. -->
										<MenuButton
											stopPropagationOnClick={true}
											on:click={() => aiChatManager.toggleOpen()}
											{isCollapsed}
											icon={WandSparkles}
											iconProps={{ forceDarkMode: true }}
											label="Ask AI"
											class="!text-xs"
											iconClasses="!text-ai"
											shortcut={`${getModifierKey()}L`}
										/>
									{/if}
								</div>

								<SidebarContent
									{isCollapsed}
									showSecondary={false}
									excludeMainLabels={['Home', 'Runs']}
									numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
										? 0
										: numUnacknowledgedCriticalAlerts}
								/>
							{/if}

							<!-- Settings dropdown stays across both modes; session mode just hides
							     the workspace-settings entry. -->
							<div class="px-2 pb-1">
								<SettingsMenu {isCollapsed} hideWorkspaceSettings={sessionMode} />
							</div>

							<div class="flex-shrink-0 flex px-4 pb-3.5">
								<button
									onclick={() => {
										isCollapsed = !isCollapsed
										// Manual toggle is the persisted preference (auto-collapse isn't).
										collapsePref.val = isCollapsed
									}}
								>
									<ArrowLeft
										size={16}
										class={classNames(
											'flex-shrink-0 h-4 w-4 transition-all ease-in-out duration-200 text-secondary',
											isCollapsed ? 'rotate-180' : 'rotate-0'
										)}
									/>
								</button>
							</div>
						</div>
					</div>
				{/if}
			{:else}
				<div class="absolute top-1 left-1 z5000">
					<OperatorMenu favoriteLinks={favoriteManager.current} />
				</div>
			{/if}
			<!-- Legacy menu -->
			<div
				class={classNames(
					'fixed inset-0 bg-black/50 transition-opacity ease-linear duration-300',
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
						<div class="h-full" style:background-color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}>
							<div
								class="flex gap-x-2 flex-shrink-0 p-4 font-semibold text-emphasis w-10"
								class:w-40={!isCollapsed}
							>
								<WindmillIcon white={darkMode} height="20px" width="20px" />
								{#if !isCollapsed}{#if $whitelabelNameStore}{capitalize(
											$whitelabelNameStore
										)}{:else}Windmill{/if}{/if}
							</div>

							<div class="px-2 py-4 space-y-2 border-y border-light dark:border-gray-700">
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
									iconClasses="!text-ai"
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
			{#if $userStore?.is_service_account}
				<div
					class="bg-yellow-100 dark:bg-yellow-900/50 border-b border-yellow-300 dark:border-yellow-700 px-4 py-2 text-sm text-yellow-800 dark:text-yellow-200 flex items-center justify-center gap-4 shrink-0"
				>
					<span>
						Viewing workspace on behalf of <strong>{$userStore.username}</strong>
						<span class="text-yellow-600 dark:text-yellow-400"
							>(impersonated by {$userStore.impersonating_email})</span
						>
					</span>
					<button
						class="px-3 py-1 text-xs font-medium bg-yellow-200 dark:bg-yellow-800 hover:bg-yellow-300 dark:hover:bg-yellow-700 rounded transition-colors"
						onclick={async () => {
							const savedToken = sessionStorage.getItem('pre_impersonation_token')
							if (savedToken && $workspaceStore) {
								try {
									await UserService.exitImpersonation({
										workspace: $workspaceStore,
										requestBody: { token: savedToken }
									})
								} catch (e) {
									console.error('Failed to exit impersonation', e)
								}
								sessionStorage.removeItem('pre_impersonation_token')
								sessionStorage.removeItem('pre_impersonation_email')
							}
							window.location.href = '/workspace_settings?tab=users'
						}}
					>
						Exit impersonation
					</button>
				</div>
			{/if}
			<AiChatLayout
				{children}
				noPadding={devOnly || menuHidden}
				disableAi={globalAiEnabled ? true : sessionMode}
				{isCollapsed}
				isMobile={innerWidth < 768}
				onMenuOpen={() => {
					menuOpen = true
				}}
			/>
		</div>
	</div>
{:else}
	<CenteredModal title="Loading user..." loading={true}></CenteredModal>
{/if}

{#if $workspaceStore && globalDbManagerDrawer.val}
	<DBManagerDrawer uriState={globalDbManagerDrawer.val} />
{/if}

<ForkConflictModal />

<Modal2
	title="Forking {$workspaceStore}"
	target="#content"
	fixedHeight="lg"
	fixedWidth="sm"
	contentClasses="flex-col"
	bind:isOpen={() => !!globalForkModal.val?.opened, (v) => !v && (globalForkModal.val = undefined)}
>
	{#if globalForkModal.val}
		<CreateWorkspaceInner isFork onFinish={() => (globalForkModal.val = undefined)} />
	{/if}
</Modal2>
