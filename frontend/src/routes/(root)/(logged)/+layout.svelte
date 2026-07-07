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
	import SidebarScrollArea from '$lib/components/sidebar/SidebarScrollArea.svelte'
	import { SIDEBAR_BG, SIDEBAR_BG_DARK } from '$lib/components/sidebar/sidebarChrome'
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
	import { goto, setChatNavigateHandler } from '$lib/navigation'
	import { registerToolDisplayActionHandler } from '$lib/components/copilot/chat/createdResourceActions.svelte'
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
	import { PanelLeftClose, PanelLeftOpen, Home, Play, Search, WandSparkles } from 'lucide-svelte'
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
	import { onDestroy, setContext, untrack } from 'svelte'
	import { base } from '$app/paths'
	import { Menubar } from '$lib/components/meltComponents'
	import { aiChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import AiChatLayout from '$lib/components/copilot/chat/AiChatLayout.svelte'
	import SessionPicker from '$lib/components/sessions/SessionPicker.svelte'
	import SessionModeSwitch from '$lib/components/sessions/SessionModeSwitch.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { parsePreviewItemRoute } from '$lib/components/sessions/previewRouter'
	import { rememberNavRoute } from '$lib/components/sessions/sessionSwitch.svelte'
	import { sessionState } from '$lib/components/sessions/sessionState.svelte'
	import { currentWorkspaceRootId } from '$lib/components/sessions/sessionScope.svelte'
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
	// Persisted nav-rail collapse preference. A deliberate collapse writes to it —
	// the manual toggle and a drag past the collapse threshold. The contextual
	// auto-collapse (app-mode routes, narrow widths) mutates the in-memory
	// `isCollapsed` without persisting, so it stays transient and never gets
	// "stuck" collapsed across reloads.
	const collapsePref = useLocalStorageValue<boolean>('nav_menu_collapsed', false, 'boolean')
	let isCollapsed = $state(collapsePref.val)

	// Resizable desktop rail, sized in REM so it scales with the root font-size the
	// same way the old `w-52`/`w-12` classes did — `:root` jumps to 18px past 1760px
	// wide (app.css), which grows the rem-based button content; a fixed-px rail would
	// not grow with it and the content would overflow. SIDEBAR_MIN_REM is the default
	// expanded width (the old w-52); the handle only resizes when expanded and only
	// widens from there — collapsing is the toggle button's job, not the drag's.
	const SIDEBAR_MIN_REM = 13
	const SIDEBAR_COLLAPSED_REM = 3
	// Root font-size in px, used to convert the pointer's clientX (px) into rem.
	function rootFontPx(): number {
		if (!BROWSER) return 16
		const px = parseFloat(getComputedStyle(document.documentElement).fontSize)
		return Number.isFinite(px) && px > 0 ? px : 16
	}
	const widthPref = useLocalStorageValue<number>('nav_menu_width_rem', SIDEBAR_MIN_REM, 'number')
	// A non-finite stored width (e.g. NaN from a corrupt entry) must never reach the
	// `style:width` binding: `Math.max(13, NaN)` is NaN, which renders as the invalid
	// `width: NaNrem` and collapses the fixed rail to content width, mangling every
	// menu button. Clamp defensively.
	function clampSidebarWidth(v: number): number {
		return Number.isFinite(v) ? Math.max(SIDEBAR_MIN_REM, v) : SIDEBAR_MIN_REM
	}
	const initialSidebarWidth = clampSidebarWidth(widthPref.val)
	let sidebarWidth = $state(initialSidebarWidth)
	// Heal a corrupt stored value so it stops breaking the rail on every reload.
	if (widthPref.val !== initialSidebarWidth) widthPref.val = initialSidebarWidth
	let resizingSidebar = $state(false)
	// Width (in rem) the content offset must track: the icon strip when collapsed,
	// the user-chosen width otherwise.
	let railWidth = $derived(isCollapsed ? SIDEBAR_COLLAPSED_REM : sidebarWidth)
	// Width transition shared by the rail and the content offset: none for the whole
	// drag (the rail tracks the pointer 1:1 and hits the min as a hard wall, no
	// friction), a plain ease only for the collapse/expand toggle.
	let sidebarTransitionClass = $derived(
		resizingSidebar ? '' : 'transition-all duration-200 ease-in-out'
	)
	// Set while a drag is live so it can be torn down if the layout unmounts
	// mid-drag (otherwise the window listeners would leak).
	let stopSidebarResize: (() => void) | null = null

	function startSidebarResize(e: PointerEvent) {
		// One drag at a time: ignore a second concurrent pointer (multi-touch on the
		// handle) so its window listeners can't outlive the first pointer's stop().
		if (resizingSidebar) return
		e.preventDefault()
		const handle = e.currentTarget as HTMLElement
		// Capture the pointer so events keep flowing to us even when the cursor
		// crosses an iframe in the content area (app/session previews) — without
		// this the drag silently stalls the moment it enters the iframe.
		try {
			handle.setPointerCapture(e.pointerId)
		} catch {}
		resizingSidebar = true
		// The rail is fixed at left:0, so the pointer's clientX is the width — in px.
		// Convert to rem (the unit the rail is sized in) via the root font-size. Pure
		// resize: clamp at the min so the rail stops there like a wall (dragging left
		// never collapses — that's the toggle button's job).
		const onMove = (ev: PointerEvent) => {
			sidebarWidth = Math.max(SIDEBAR_MIN_REM, ev.clientX / rootFontPx())
		}
		// pointercancel (and unmount, via onDestroy) must clear the state too, or
		// `resizingSidebar` sticks true — the overlay and handle highlight stay up
		// and the window listeners leak.
		const stop = () => {
			if (!resizingSidebar) return
			resizingSidebar = false
			widthPref.val = sidebarWidth
			window.removeEventListener('pointermove', onMove)
			window.removeEventListener('pointerup', stop)
			window.removeEventListener('pointercancel', stop)
			try {
				handle.releasePointerCapture(e.pointerId)
			} catch {}
			stopSidebarResize = null
		}
		stopSidebarResize = stop
		window.addEventListener('pointermove', onMove)
		window.addEventListener('pointerup', stop)
		window.addEventListener('pointercancel', stop)
	}

	onDestroy(() => stopSidebarResize?.())

	// Give router-free callers (AI chat tools) a way to navigate: a direct-nav slot
	// for in-context moves and the 'navigate' display-action for cross-page link chips.
	$effect(() => {
		setChatNavigateHandler((path) => goto(path))
		const unregisterNavigate = registerToolDisplayActionHandler('navigate', (action) => {
			if (action.type === 'navigate') goto(action.url)
		})
		return () => {
			setChatNavigateHandler(undefined)
			unregisterNavigate()
		}
	})
	let userSettings: UserSettings | undefined = $state()
	let superadminSettings: SuperadminSettings | undefined = $state()
	let menuHidden = $state(false)
	let isDarkMode = useIsDarkMode()
	let darkMode = $derived(isDarkMode.val)

	// Session mode is route-derived: the rail shows the sessions sidebar on the
	// /sessions page and the workspace navigation everywhere else. The switch
	// (SessionModeSwitch) just navigates in and out of that route.
	let sessionMode = $derived(page.url.pathname.startsWith(base + '/sessions'))
	// Session mode points the bottom settings entry at the open session's own
	// workspace. An unsent draft hasn't committed one yet, so it falls back to
	// the family root.
	let sessionSettingsWorkspace = $derived.by(() => {
		if (!sessionMode) return undefined
		const current = sessionState.sessions.find((s) => s.id === sessionState.currentSessionId)
		return current?.workspace_id ?? $currentWorkspaceRootId ?? $workspaceStore ?? undefined
	})
	// Inside a preview iframe the rail still renders (navigation mode), but the
	// switch must not — entering session mode from within the preview would
	// nest the whole experience. Hide it when embedded.
	const embedded = BROWSER && window.self !== window.top

	// AI sessions are still dev-gated (localStorage wm_dev_global_ai=1), same as
	// the global chat. The Workspace ⇄ Sessions switch is the only entry point, so
	// gate it on the flag too — otherwise it would ship the unfinished experience
	// to prod. The /sessions page has its own gate for direct navigation.
	const globalAiEnabled = isGlobalAiEnabled()

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
			// A preview iframe shares BOTH localStorage and sessionStorage with the
			// top-level app (same-origin nested browsing contexts share the top-level
			// session storage). Persisting its session-scoped workspace to either would
			// clobber the workspace the user is actually navigating. Keep it in-memory
			// only ($workspaceStore is still set from the ?workspace= param for the
			// iframe's own API calls); the fork survives iframe reloads because the
			// preview always reloads a URL that carries ?workspace= (see
			// PreviewTabHost.reload). Only the top window owns the persisted keys.
			if (!embedded) {
				try {
					sessionStorage.setItem('workspace', String(workspace))
					localStorage.setItem('workspace', String(workspace))
				} catch (e) {
					console.error('Could not persist workspace to local storage', e)
				}
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
			// Populate for all members (not just admins) so non-admin developers also get premium-gated
			// affordances like the fork entry points on cloud. The `is_premium` endpoint is a boolean
			// and no longer admin-gated. Best-effort: a failure here must not block user-store init.
			if (isCloudHosted()) {
				try {
					isPremiumStore.set(await WorkspaceService.getIsPremium({ workspace }))
				} catch (e) {
					console.error('Could not fetch premium status', e)
				}
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

	// A job-detail navigation (/run/<id>) inside a preview tab should open the job in
	// a NEW tab rather than navigate the current tab away from its page (e.g. clicking
	// a job in the Runs tab keeps Runs put and opens the run beside it). Returns the
	// href to open (nomenubar dropped — the preview host re-adds it) and a short label.
	function previewRunTarget(url: URL | undefined): { href: string; label: string } | undefined {
		if (!url) return undefined
		const m = url.pathname.match(/\/run\/([^/?#]+)/)
		if (!m) return undefined
		const u = new URL(url.href)
		u.searchParams.delete('nomenubar')
		const id = decodeURIComponent(m[1])
		return { href: u.pathname + u.search, label: `Run ${id.slice(0, 8)}` }
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
			const runTarget = previewRunTarget(navigation.to?.url)
			if (runTarget) {
				navigation.cancel()
				try {
					window.parent.postMessage(
						{ type: 'wm.session.openRun', href: runTarget.href, label: runTarget.label },
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

<!-- Windmill brand mark anchoring the sidebar bottom (the header slot is taken
     by the workspace picker). -->
{#snippet brandMark(collapsed: boolean)}
	<div class="flex items-center gap-x-1.5 text-xs font-semibold text-emphasis">
		<WindmillIcon white={darkMode} height="16px" width="16px" />
		{#if !collapsed}
			{$whitelabelNameStore ? capitalize($whitelabelNameStore) : 'Windmill'}
		{/if}
	</div>
{/snippet}

<!-- Settings / instance section. Shared by both sidebar surfaces and both modes
     (in nav mode it's the last section inside the unified scroll area; in session
     mode it stays pinned under the session list). -->
{#snippet settingsMenu(collapsed: boolean)}
	<div class="px-2 pb-1">
		<SettingsMenu
			isCollapsed={collapsed}
			workspaceSettingsTarget={sessionSettingsWorkspace}
			numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
				? 0
				: numUnacknowledgedCriticalAlerts}
		/>
	</div>
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
		{#if resizingSidebar}
			<!-- While dragging, this overlay gives the whole viewport the resize cursor
			     and stops the pointer from selecting text or hovering content/iframes. -->
			<div class="fixed inset-0 z-50 cursor-col-resize select-none"></div>
		{/if}
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
										{@render settingsMenu(false)}
									{:else}
										<!-- Navigation mode: Home → Settings scroll as ONE block with an
										     overflow fade hint (same treatment as the desktop rail). -->
										<SidebarScrollArea color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}>
											<!-- Section 1: Home/Runs + Favorites + Search -->
											<div class="px-2 pt-1 w-52 flex flex-col gap-1">
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

											<!-- Section 2: workspace items -->
											<SidebarContent
												isCollapsed={false}
												showSecondary={false}
												excludeMainLabels={['Home', 'Runs']}
												numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
													? 0
													: numUnacknowledgedCriticalAlerts}
											/>

											<!-- Section 3: instance settings — mt-auto drops it to the bottom
											     when there is spare room (see the desktop rail for the rationale). -->
											<div class="mt-auto">
												{@render settingsMenu(false)}
											</div>
										</SidebarScrollArea>
									{/if}

									<div class="px-4 pt-3 pb-3.5 w-52">
										{@render brandMark(false)}
									</div>
								</div>
							</div>
						</div>
					</div>
				{:else}
					<div
						id="sidebar"
						class={classNames(
							'flex flex-col fixed inset-y-0 z-40 ',
							sidebarTransitionClass,
							devOnly ? '!hidden' : ''
						)}
						style:width="{railWidth}rem"
					>
						<div
							class="flex-1 flex flex-col min-h-0 h-screen shadow-[inset_-1px_0_0_0_rgb(var(--color-border-light))] dark:shadow-[inset_-1px_0_0_0_#374151]"
							style:background-color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}
						>
							{#if !isCollapsed}
								<!-- Resize handle straddling the right edge, only while expanded:
								     drag to widen (clamped at the min). Collapsing is the toggle
								     button's job. -->
								<div
									role="separator"
									aria-orientation="vertical"
									aria-label="Resize sidebar"
									title="Drag to resize"
									class={classNames(
										'absolute inset-y-0 -right-0.5 w-1.5 cursor-col-resize z-50 transition-colors',
										resizingSidebar ? '' : 'hover:bg-surface-hover'
									)}
									onpointerdown={startSidebarResize}
								></div>
							{/if}
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
								{@render settingsMenu(isCollapsed)}
							{:else}
								<!-- Navigation mode: Home → Settings scroll as ONE block with an
								     overflow fade hint. The three sections (Home/Runs + Favorites +
								     Search / workspace items / instance settings) are spaced by the
								     scroll area's gap rather than each scrolling on its own. -->
								<SidebarScrollArea color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}>
									<!-- Section 1: Home/Runs + Favorites + Search -->
									<div class="px-2 pt-1 flex flex-col gap-1">
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

									<!-- Section 2: workspace items -->
									<SidebarContent
										{isCollapsed}
										showSecondary={false}
										excludeMainLabels={['Home', 'Runs']}
										numUnacknowledgedCriticalAlerts={isCriticalAlertsUiMuted
											? 0
											: numUnacknowledgedCriticalAlerts}
									/>

									<!-- Section 3: instance settings — mt-auto drops it to the bottom
									     when the column has spare room, and collapses back to the normal
									     inter-section gap once the content overflows and scrolls. -->
									<div class="mt-auto">
										{@render settingsMenu(isCollapsed)}
									</div>
								</SidebarScrollArea>
							{/if}

							<div
								class="flex-shrink-0 flex pt-3 pb-3.5 {isCollapsed
									? 'flex-col items-center gap-3'
									: 'items-center justify-between px-4'}"
							>
								{@render brandMark(isCollapsed)}
								<!-- p-2/-m-2 widens the hit area to ~32px without moving the icon. -->
								<button
									class="p-2 -m-2 rounded hover:bg-surface-hover"
									title={isCollapsed ? 'Expand sidebar' : 'Collapse sidebar'}
									onclick={() => {
										isCollapsed = !isCollapsed
										// Manual toggle is the persisted preference (auto-collapse isn't).
										collapsePref.val = isCollapsed
									}}
								>
									{#if isCollapsed}
										<PanelLeftOpen size={14} class="flex-shrink-0 h-3.5 w-3.5 text-hint" />
									{:else}
										<PanelLeftClose size={14} class="flex-shrink-0 h-3.5 w-3.5 text-hint" />
									{/if}
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
				sidebarWidth={railWidth}
				transitionClass={sidebarTransitionClass}
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
	fixedHeight="adaptive"
	fixedWidth="sm"
	contentClasses="flex-col"
	bind:isOpen={() => !!globalForkModal.val?.opened, (v) => !v && (globalForkModal.val = undefined)}
>
	{#if globalForkModal.val}
		<CreateWorkspaceInner isFork inModal onFinish={() => (globalForkModal.val = undefined)} />
	{/if}
</Modal2>
