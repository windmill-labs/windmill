<script lang="ts">
	import { onMount, tick, untrack } from 'svelte'
	import { SvelteSet } from 'svelte/reactivity'
	import { page } from '$app/state'
	import {
		Plus,
		Maximize2,
		Minimize2,
		ExternalLink,
		PanelRightClose,
		PanelRightOpen,
		ChevronDown,
		MonitorPlay,
		Loader2
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Button } from '$lib/components/common'
	import DraggableTabs, { type TabItem } from '$lib/components/common/tabs/DraggableTabs.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PreviewRouterPicker, {
		type Scope
	} from '$lib/components/sessions/PreviewRouterPicker.svelte'
	import { goto } from '$lib/navigation'
	import { resource } from 'runed'
	import { WorkspaceService } from '$lib/gen'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import PreviewTabHost from '$lib/components/sessions/PreviewTabHost.svelte'
	import { useIsDarkMode } from '$lib/components/DarkModeObserver.svelte'
	import {
		createSession,
		findEmptyLandingSession,
		getEffectiveWorkspaceId,
		isTearingDownOpenSession,
		selectSession,
		sessionInCurrentFamily,
		sessionState,
		type SessionPreviewTab
	} from '$lib/components/sessions/sessionState.svelte'
	import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
	import { enterSessionMode } from '$lib/components/sessions/sessionSwitch.svelte'
	import type { SessionPreviewTabs } from '$lib/components/sessions/sessionPreviewTabs.svelte'
	import { userStore, userWorkspaces, usersWorkspaceStore, workspaceStore } from '$lib/stores'
	import {
		getOrCreateRuntime,
		getRuntime,
		listRuntimes
	} from '$lib/components/sessions/sessionRuntime.svelte'
	import { markSessionSeen } from '$lib/components/sessions/sessionUnread.svelte'
	import { markSessionRecovered } from '$lib/components/sessions/sessionRecoveryNotice.svelte'
	import {
		isGlobalAiEnabled,
		setSessionsBetaOptOut
	} from '$lib/components/copilot/chat/global/gate'
	import { setToolCompletionListener } from '$lib/components/copilot/chat/shared'
	import { registerToolDisplayActionHandler } from '$lib/components/copilot/chat/createdResourceActions.svelte'
	import { previewTargetForSessionTarget } from '$lib/components/sessions/sessionPreviewTabs.svelte'
	import { base } from '$lib/base'
	import {
		artifactKey,
		itemDisplayName,
		matchPreviewPage,
		pageKey,
		parseArtifactRoute,
		parsePreviewItemRoute,
		previewLocationLabel,
		type PreviewTarget
	} from '$lib/components/sessions/previewRouter'
	import { toolReloadEffect, tabsToReload } from '$lib/components/sessions/previewReload'
	import {
		leafKeyFor,
		loadKind,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'

	const globalEnabled = isGlobalAiEnabled()

	// One observer shared by every tab host, which mirrors it into page iframes
	// (see PreviewTabHost).
	const isDarkMode = useIsDarkMode()

	// The sessions page hosts preview iframes that load Windmill pages. If one of
	// those iframes navigates back to /sessions, mounting the full UI again would
	// nest another sessions page (with its own iframes) endlessly. Detect the
	// iframe context and refuse to mount when embedded.
	const embedded = typeof window !== 'undefined' && window.self !== window.top

	// Warm the lazily-loaded editor views (see PreviewTabHost) once the page is
	// idle: entering session mode stays instant, and by the time the user opens
	// an editor tab its chunk is usually already cached. Sequential so the
	// prefetch trickles instead of fanning out four heavy graphs at once.
	$effect(() => {
		if (embedded || !globalEnabled) return
		// Once the chain has started, cancelling the idle handle no longer helps —
		// the disposed check between imports is what stops a user who left session
		// mode from pulling the remaining graphs on whatever page they went to.
		// (An import already in flight can't be aborted; only the tail is skipped.)
		let disposed = false
		const prefetch = async () => {
			const loaders = [
				() => import('$lib/components/sessions/ScriptEditorView.svelte'),
				() => import('$lib/components/sessions/FlowEditorView.svelte'),
				() => import('$lib/components/sessions/RawAppEditorView.svelte'),
				() => import('$lib/components/sessions/PipelineEditorView.svelte')
			]
			for (const load of loaders) {
				if (disposed) return
				await load()
			}
		}
		// Best-effort warming: swallow chunk-load failures — the {#await} on the
		// actual open path surfaces (and retries) them.
		const run = () => void prefetch().catch(() => {})
		const hasIdle = 'requestIdleCallback' in window
		const handle = hasIdle ? window.requestIdleCallback(run) : window.setTimeout(run, 2000)
		return () => {
			disposed = true
			if (hasIdle) window.cancelIdleCallback(handle)
			else window.clearTimeout(handle)
		}
	})

	const sessionName = $derived(page.url.searchParams.get('session_name') ?? '')

	// Unfiltered resolution by name: drives the recovery effect below and the
	// active-session lookup.
	const sessionByName = $derived(
		sessionName ? sessionState.sessions.find((s) => s.name === sessionName) : undefined
	)

	// Opening a session deliberately does NOT switch the global workspace: the
	// chat targets the session's own workspace via the manager's workspace
	// resolver, so the user's active (navigation-mode) workspace is left alone.

	// Resolve by name without applying the sidebar scope filter so an open
	// chat survives within-family workspace switches.
	const activeSession = $derived(sessionState.sessions.find((s) => s.name === sessionName))

	// Family reconcile: a workspace switch can land this page with no session
	// selected or with another family's session in the URL (the sidebar picker's
	// link navigation keeps the route), and a chat must not bleed across
	// families. Re-enter session mode scoped to the active family: keep the open
	// chat when it belongs there, else its most recent active session, else a
	// fresh one. A `session_name` that resolves to nothing is left to the
	// recovery effect below.
	$effect(() => {
		if (embedded || !sessionState.hydrated) return
		// Family membership can't be judged before the workspace list arrives:
		// workspaceRootId falls back to the raw id for workspaces it can't find,
		// which makes a same-family session look foreign on a hard reload and
		// would bounce the URL to another (or a brand-new) session.
		if ($usersWorkspaceStore === undefined) return
		// sessionInCurrentFamily reads these via get(), so track them explicitly.
		$workspaceStore
		$userWorkspaces
		const current = activeSession
		const shouldReenter = current ? !sessionInCurrentFamily(current) : !sessionName
		if (!shouldReenter) return
		untrack(() => void enterSessionMode({ replace: true }))
	})

	// Held across the swap so the session layout mounts once, after the navigation
	// settles. A recovered session often takes the very name that was missing, so
	// the URL resolves before `goto` returns; mounting the panes mid-navigation
	// makes Splitpanes miss the collapsed pane's `maxSize: 0` and leave it open.
	let recovering = $state(false)

	// Bounds the wait for the workspace list the guard above needs. The list can
	// fail to arrive for good, and a spinner that never resolves is a worse
	// outcome than the spare blank that judging family membership early leaves.
	const WORKSPACE_LIST_GRACE_MS = 3000
	let workspaceListOverdue = $state(false)
	onMount(() => {
		const handle = setTimeout(() => (workspaceListOverdue = true), WORKSPACE_LIST_GRACE_MS)
		return () => clearTimeout(handle)
	})

	// The URL names a session this browser doesn't hold. Land on an empty one,
	// never the most recent: an existing conversation would read as a successful
	// load. `recovering` also guards re-entry: recovery mutates the session list
	// this effect tracks, while the URL that would stop it only updates on `goto`.
	$effect(() => {
		if (embedded || !globalEnabled || $userStore?.operator) return
		if (!sessionState.hydrated || recovering) return
		// A deliberate delete removes the open session ahead of its own navigation.
		// Claiming that gap would take over the URL and tell the user the session
		// they just deleted couldn't be found.
		if (isTearingDownOpenSession()) return
		if ($usersWorkspaceStore === undefined && !workspaceListOverdue) return
		if (!sessionName || sessionByName) return
		untrack(() => void recoverToNewSession())
	})

	// Touch the runtime for the active session so it gets created on first visit
	// and the pane shows up. Subsequent renders find it via listRuntimes().
	//
	// Gate on session identity (id) rather than the full activeSession
	// derived — sessionState.sessions mutates on every persisted change
	// (including token-by-token last_message updates during AI streaming),
	// so a value-trigger would re-run dozens of times per turn. We only
	// want to react when the user actually arrives at a new session.
	let lastArrivedSessionId: string | undefined
	$effect(() => {
		const session = activeSession
		if (!session) {
			lastArrivedSessionId = undefined
			return
		}
		if (lastArrivedSessionId === session.id) return
		lastArrivedSessionId = session.id
		untrack(() => {
			// Keep currentSessionId in sync with the URL so consumers react to
			// deep links the same way they react to picker clicks.
			selectSession(session.id)
			getOrCreateRuntime(session)
		})
	})

	// Warm = sessions with a live runtime. The picker eagerly creates runtimes
	// for its visible sessions, so this tracks whatever it shows. Keeping warm
	// chats mounted (stacked, visibility-toggled) preserves their scroll/draft
	// state across switches.
	const warmSessions = $derived(
		listRuntimes()
			.map((r) => sessionState.sessions.find((s) => s.id === r.sessionId))
			.filter((s): s is NonNullable<typeof s> => s != null)
	)

	// Mark the active session "seen" up to its current message count: arrive →
	// clear unread; AI streams a new message while we're here → clear again. The
	// effect depends only on the length, not the array contents, so token-by-token
	// streams within a single message don't fire it on every chunk.
	$effect(() => {
		const id = activeSession?.id
		if (!id) return
		const rt = getRuntime(id)
		if (!rt) return
		const count = rt.manager.displayMessages.length
		untrack(() => markSessionSeen(id, count))
	})

	async function recoverToNewSession() {
		recovering = true
		try {
			// Prefer an existing empty session over stacking another blank onto the
			// sidebar: createSession reuses only its own in-memory drafts, so one
			// persisted by any other touch (a workspace pick, a preview tab) would
			// be passed over.
			const target = findEmptyLandingSession() ?? createSession()
			selectSession(target.id)
			markSessionRecovered(target.id)
			await goto(`/sessions?session_name=${encodeURIComponent(target.name)}`, {
				replaceState: true
			})
			await tick()
		} finally {
			recovering = false
		}
	}

	// Preview panel: a tiny tabbed browser over Windmill. Every tab stays mounted
	// (stacked, visibility-toggled, like the warm chat sessions) so switching
	// preserves each previewed page's scroll/edit state. The tab model lives on
	// the active session's runtime (previewTabs) — one live copy that both this
	// page (renderer) and the chat's open_preview tool drive — so there's no
	// page-local tab state to seed from IndexedDB or reconcile against the tool.
	//
	// Pure read (not getOrCreateRuntime): creating a runtime mutates the global
	// `runtimes` map, which is forbidden inside a $derived. The arrival effect
	// creates it in effect context; getRuntime reads the SvelteMap reactively so
	// this re-derives the moment it lands.
	const activeRuntime = $derived(activeSession ? getRuntime(activeSession.id) : undefined)
	const owner = $derived(activeRuntime?.previewTabs)

	// The workspace the active session acts on. Preview pickers load their items
	// from it and "Open in workspace" targets it, so a fork-scoped session never
	// lists or opens against the navigation workspace ($workspaceStore).
	const previewWorkspace = $derived(
		(activeSession ? getEffectiveWorkspaceId(activeSession) : undefined) ?? $workspaceStore
	)

	// Whether that workspace hid the AI assistant (`ai_config.copilot_disabled`). Read
	// directly rather than from `copilotInfo`: that store holds whichever workspace loaded
	// last, and the gate below unmounts the session wrapper that would refresh it, so a
	// hidden verdict would stick across session and workspace switches. Tagged with its
	// workspace and guarded against a superseded response like the protection-rules
	// resource: runed keeps the previous `current` while a new source loads, so a switch
	// would otherwise be judged on the previous workspace's verdict. A failed read leaves
	// the page usable, as an unloaded config does everywhere else.
	const workspaceAiHidden = resource(
		() => previewWorkspace,
		async (workspace, _prev, { signal }) => {
			if (!workspace) return { workspace, hidden: false }
			let hidden = false
			try {
				hidden = (await WorkspaceService.getCopilotInfo({ workspace })).copilot_disabled === true
			} catch (e) {
				console.error(`Failed to read the AI config of workspace ${workspace}:`, e)
			}
			// The generated client can't take an abort signal, so drop a superseded response here.
			if (signal.aborted) throw new DOMException('superseded', 'AbortError')
			return { workspace, hidden }
		}
	)
	// Only a verdict for the workspace currently judged counts; `undefined` means it has not
	// landed yet.
	const aiHiddenVerdict = $derived.by(() => {
		const current = workspaceAiHidden.current
		return current && current.workspace === previewWorkspace ? current.hidden : undefined
	})

	// Lazy-mount gate: a tab's content only renders once its key lands here (on
	// first activation) — so restoring a session with N saved tabs boots just
	// the active one instead of N full Windmill apps at once. Pure "has this
	// content been created yet" DOM bookkeeping, so it stays page-local while
	// the owner holds the tab identity. Keys are session-scoped (tab ids alone
	// collide across sessions — every session seeds a 'session' tab id) and the
	// set survives session switches: a warm session's mounted tabs stay alive
	// while another session is shown, same as its stacked chat column.
	//
	// MRU-capped at MAX_MOUNTED_TABS, shared across sessions: each mounted tab
	// is a full Windmill app (iframe) or live editor — without a cap warm
	// sessions accumulate one per tab ever activated. An evicted tab stays in
	// the strip and simply remounts on its next activation, same as the
	// lazy-mount path.
	const MAX_MOUNTED_TABS = 5
	const mountedTabKeys = new SvelteSet<string>()
	const tabKey = (sessionId: string, tabId: string) => `${sessionId}:${tabId}`
	const sessionOfKey = (key: string) => key.slice(0, key.indexOf(':'))
	function mountTab(key: string) {
		// Delete-then-add moves the key to the MRU end; evict from the LRU front,
		// never the tab just activated.
		mountedTabKeys.delete(key)
		mountedTabKeys.add(key)
		while (mountedTabKeys.size > MAX_MOUNTED_TABS) {
			const oldest = mountedTabKeys.values().next().value
			if (oldest === undefined || oldest === key) break
			mountedTabKeys.delete(oldest)
		}
	}
	// Mount the active session's active tab whenever either changes. Background
	// sessions' activeId changes (a chat tool opening a tab) don't mount — their
	// tabs boot lazily on first visible activation. A collapsed preview mounts
	// nothing: the pane is zero-width, so booting a full Windmill app the user
	// can't see is wasted — it mounts on expand, when previewCollapsed flips.
	$effect(() => {
		const sid = activeRuntime?.sessionId
		const activeId = owner?.activeId
		if (!sid || !activeId || previewCollapsed) return
		untrack(() => mountTab(tabKey(sid, activeId)))
	})
	// A disposed runtime unmounts its hosts; drop its keys too, else a later
	// re-open would boot every previously-mounted tab at once instead of
	// lazily, and stale keys would squat the shared MRU budget.
	$effect(() => {
		const warm = new Set(listRuntimes().map((r) => r.sessionId))
		untrack(() => {
			for (const key of [...mountedTabKeys]) {
				if (!warm.has(sessionOfKey(key))) mountedTabKeys.delete(key)
			}
		})
	})

	function selectTab(id: string) {
		owner?.select(id)
		const sid = activeRuntime?.sessionId
		if (sid) mountTab(tabKey(sid, id))
		activeTabPickerOpen = false
	}
	function openInNewTab(target: PreviewTarget) {
		owner?.open(target)
	}
	function closeTab(id: string) {
		owner?.close(id)
		const sid = activeRuntime?.sessionId
		if (sid) mountedTabKeys.delete(tabKey(sid, id))
		// The active tab is excluded from the picker's pointerdown-outside (so a
		// label click can toggle it); without this, closing the active tab would
		// carry the open picker over to the newly active one.
		activeTabPickerOpen = false
	}
	function reorderTabs(next: TabItem[]) {
		owner?.reorder(next.map((t) => t.id))
	}
	// Adapt the session tab model to DraggableTabs items (labels derived from the
	// observed location; every tab closable, none pinned).
	const previewTabItems = $derived<TabItem[]>(
		(owner?.tabs ?? []).map((t) => ({
			id: t.id,
			label: tabLabelFor(t, previewWorkspace ?? ''),
			title: tabTitleFor(t, previewWorkspace ?? '')
		}))
	)
	let newTabOpen = $state(false)
	// Separate open flag for the empty-state launcher: it can be mounted at the
	// same time as the tab-strip "+" popover, so sharing one flag would open both
	// pickers at once.
	let emptyStateNewTabOpen = $state(false)

	let fullscreen = $state(false)
	// Fullscreen is page state, not per-session, so it outlives a session switch —
	// tell the incoming session's model, whose own collapsed flag it overrides, or
	// re-opening the item plainly on screen would be judged invisible and not flash.
	$effect(() => {
		owner?.setFullscreen(fullscreen)
	})
	// Collapse the preview panel to give the chat the full width. Per-session and
	// owned by the runtime's previewTabs (restored on switch, written back on
	// toggle) so it survives session switches with the rest of the tab model.
	const previewCollapsed = $derived(owner?.collapsed ?? false)

	// Collapse works by resizing the preview pane to zero, never unmounting it:
	// the pane hosts every warm session's preview tabs, and unmounting would
	// destroy them all whenever the active session's preview is collapsed (the
	// default for a session with no tabs). Both panes are driven together — a
	// given size on one pane against a stale size on the other makes Splitpanes
	// normalize (e.g. 50 vs 100 lands at 33%), drifting on every toggle.
	// null = let Splitpanes auto-distribute (initial even split).
	let previewPaneSize = $state<number | null>(null)
	let chatPaneSize = $state<number | null>(null)
	// Even split for a session with no saved width. Effect A's seed and effect B's
	// write-back-skip guard must share this exact value, or B persists the default
	// and breaks the never-resized (undefined) invariant.
	const DEFAULT_SPLIT = 50
	let lastExpandedPreviewSize = DEFAULT_SPLIT
	// Which owner previewPaneSize is currently seeded for. The Pane is shared across
	// warm sessions, so we reseed the expanded width when the active session changes.
	let seededOwner: SessionPreviewTabs | undefined = undefined

	// Effect A — layout: reseed on session switch, then apply collapse/fullscreen.
	$effect(() => {
		const o = owner
		const collapsed = previewCollapsed
		const full = fullscreen
		untrack(() => {
			const switched = o !== seededOwner
			if (switched) {
				seededOwner = o
				// Read the saved size UNTRACKED: this must not re-run when effect B
				// writes it back, or the two effects loop.
				lastExpandedPreviewSize = o?.previewSize ?? DEFAULT_SPLIT
				// Seed the pane for the incoming session on the switch frame. The
				// collapsed case seeds 0, so the capture below never captures the
				// outgoing session's leftover width as this session's.
				previewPaneSize = collapsed ? 0 : lastExpandedPreviewSize
			}
			// effect A doesn't track previewPaneSize, so a drag never re-runs it: this is
			// the only place the live width is saved before a sentinel (collapse→0 /
			// fullscreen→100) overwrites it. The switch-frame value is the seed, not a drag.
			if (!switched && previewPaneSize && previewPaneSize > 0 && previewPaneSize < 100) {
				lastExpandedPreviewSize = previewPaneSize
			}
			if (full) {
				// Chat pane is unmounted: the preview is the only pane and must own
				// the full width, not its remembered split share.
				previewPaneSize = 100
			} else if (collapsed) {
				previewPaneSize = 0
				chatPaneSize = 100
			} else {
				if (previewPaneSize === 0 || previewPaneSize === 100 || previewPaneSize === null) {
					previewPaneSize = lastExpandedPreviewSize
				}
				chatPaneSize = 100 - previewPaneSize
			}
		})
	})

	// Effect B — write-back: persist a genuine user-dragged width to the model.
	$effect(() => {
		const size = previewPaneSize
		untrack(() => {
			// Skip when size still matches the model's saved width, or the 50 default
			// for a never-resized session (owner.previewSize === undefined): effect A's
			// reseed sets previewPaneSize to exactly that, and persisting it would
			// materialize the default and lose the "never resized" (undefined) state.
			if (
				!previewCollapsed &&
				!fullscreen &&
				size != null &&
				size > 0 &&
				size < 100 &&
				size !== (owner?.previewSize ?? DEFAULT_SPLIT)
			) {
				owner?.setPreviewSize(size)
			}
		})
	})

	// Page path shown after the workspace breadcrumb — the active tab's observed
	// location, so the breadcrumb tracks where the user browses inside the tab.
	const displayPath = $derived(owner?.activeTab?.loc ?? owner?.activeTab?.url ?? `${base}/`)
	// Artifacts have no workspace page, so "Open in workspace" can't resolve for them.
	const activeArtifact = $derived(owner?.activeTab ? parseArtifactRoute(owner.activeTab.url) : null)
	const activeTabIsArtifact = $derived(activeArtifact != null)
	// The active session's artifacts, surfaced as an "Artifacts" branch in the
	// preview pickers.
	const sessionArtifacts = $derived(activeRuntime?.manager.artifacts.artifacts ?? [])
	// Writes to the tab's own session model: a hidden warm session's iframe can
	// finish loading while another session is shown, and its location must not
	// land on the visible session's tabs.
	function onTabLoad(tabs: SessionPreviewTabs, tab: SessionPreviewTab, frame: HTMLIFrameElement) {
		try {
			const win = frame.contentWindow
			if (!win) return
			// observeLocation canonicalizes away the injected nomenubar/workspace
			// params so the tab's `loc` stays symmetric with `url` for dedupe/display.
			// The hash is kept: on a list page it names the row whose drawer is open
			// (`/schedules#u/me/daily`), which is what tells the chat what the user
			// is looking at.
			const observe = () => {
				try {
					const loc = win.location
					tabs.observeLocation(tab.id, loc.pathname + loc.search + loc.hash)
				} catch {
					// Same best-effort as below.
				}
			}
			observe()
			// A drawer only changes the hash and a filter only rewrites the query; neither
			// reloads the frame, so `load` alone would leave `loc` frozen on the seeded page.
			// These listeners die with the framed document, so each load attaches one set.
			win.addEventListener('hashchange', observe)
			win.addEventListener('popstate', observe)
			// Filters write params with `replaceState` (shallow routing), which fires no
			// event at all — the history methods are the only way to see them. Guarded so a
			// re-load reusing the window can't wrap the wrapper.
			const w = win as Window & { __wmObservedHistory?: boolean }
			if (!w.__wmObservedHistory) {
				w.__wmObservedHistory = true
				for (const method of ['pushState', 'replaceState'] as const) {
					const original = win.history[method]
					win.history[method] = function (this: History, ...args: any[]) {
						const result = original.apply(this, args as any)
						observe()
						return result
					} as History[typeof method]
				}
			}
		} catch {
			// Best-effort: the preview is same-origin, but reading location could
			// still throw mid-navigation — keep the seeded path in that case.
		}
	}

	// Reload mounted preview tabs affected by a mutating chat tool. Item and pipeline
	// tabs are live editors that self-sync from the store the chat mutates, so nothing
	// reloads them. Only list-page tabs (schedules, resources, …) are iframes, and each
	// reloads only when a tool actually changed *its* page (toolReloadEffect) — so a
	// schedule write leaves the Resources tab alone, and a purely local tool (saving
	// user instructions) reloads nothing.
	const tabHosts: Record<string, PreviewTabHost | undefined> = {}

	let reloadHandle: ReturnType<typeof setTimeout> | undefined
	// Base-stripped list-page paths (e.g. `/schedules`) a chat round touched since
	// the last flush — see toolReloadEffect for how tools map to pages.
	let pendingPages = new Set<string>()

	// Reload the mounted list-page tabs a chat round changed, across all warm
	// sessions (a hidden preview would otherwise show pre-mutation content on
	// return). tabsToReload picks only the tabs whose page is in `pages`.
	function reloadTabs(pages: Set<string>) {
		for (const s of warmSessions) {
			const owner = getRuntime(s.id)?.previewTabs
			if (!owner) continue
			for (const tab of tabsToReload(owner.tabs, pages)) {
				const key = tabKey(s.id, tab.id)
				if (mountedTabKeys.has(key)) tabHosts[key]?.reload()
			}
		}
	}
	function flushReload() {
		const pages = pendingPages
		pendingPages = new Set()
		reloadTabs(pages)
	}
	$effect(() => {
		// Debounced so a burst of writes (the AI editing several files) reloads once.
		setToolCompletionListener((name, args) => {
			const { pages } = toolReloadEffect(name, args)
			if (pages.length === 0) return
			for (const p of pages) pendingPages.add(p)
			clearTimeout(reloadHandle)
			reloadHandle = setTimeout(flushReload, 500)
		})
		return () => {
			clearTimeout(reloadHandle)
			pendingPages = new Set()
			setToolCompletionListener(undefined)
		}
	})

	// The visible chat is always the active session, so `owner` is its panel. Read
	// `owner` lazily inside the handler (not in the effect body) so this registers
	// once, not on every session switch.
	$effect(() => {
		return registerToolDisplayActionHandler('open_item_preview', (action) => {
			if (action.type !== 'open_item_preview') return
			const o = owner
			if (!o) return
			const target = previewTargetForSessionTarget(action.previewKind, action.path)
			if (!target) return
			o.open(target)
		})
	})

	// Editor-style breadcrumb over the previewed page. We only render clickable
	// segments when the preview is sitting on a script/flow/app route — for any
	// other page (home, runs, …) there's no item to drill into, so we fall back
	// to the plain path.
	const parsedRoute = $derived(parsePreviewItemRoute(displayPath))

	// Split the item path into breadcrumb dirs + leaf, mirroring EditorHeader:
	// scope (`f/<folder>` | `u/<user>`) → subfolders → item name. Prefers the
	// tab's friendly path (a draft-only item's typed name): the picker tree
	// groups such an item under its friendly folder, so dirs derived from the
	// `…/draft_<uuid>` storage path would scope the picker into a folder the
	// item isn't displayed in.
	const segments = $derived.by(() => {
		const itemPath = owner?.activeTab?.friendlyPath ?? parsedRoute?.itemPath
		if (!itemPath) return null
		const parts = itemPath.split('/')
		if (parts.length < 3) return null
		const scope = parts.slice(0, 2).join('/')
		const slug = parts.slice(2)
		const dirs: { name: string; fullPath: string }[] = [{ name: scope, fullPath: scope }]
		let acc = scope
		for (let i = 0; i < slug.length - 1; i++) {
			acc = `${acc}/${slug[i]}`
			dirs.push({ name: slug[i], fullPath: acc })
		}
		const leaf = { name: slug[slug.length - 1], fullPath: itemPath }
		return { dirs, leaf }
	})

	const currentItem = $derived<WorkspaceItem & { savedPath?: string }>({
		path: parsedRoute?.itemPath ?? '',
		summary: '',
		kind: parsedRoute?.kind ?? 'script',
		raw_app: parsedRoute?.raw_app ?? false
	})

	// On a non-item page, identify the known workspace page so the tab shows its
	// name (e.g. "Workspace settings") and the picker highlights it.
	const currentPage = $derived(parsedRoute ? undefined : matchPreviewPage(displayPath))

	// The active tab's picker lands on its current location: an item is scoped
	// into its folder and highlighted; a known page is highlighted at root.
	const activePickerScope = $derived<Scope>(
		parsedRoute
			? segments && segments.dirs.length > 0
				? { kind: 'all', dir: segments.dirs[segments.dirs.length - 1].fullPath }
				: { kind: 'all' }
			: undefined
	)
	const activePickerHighlight = $derived(
		parsedRoute
			? leafKeyFor(parsedRoute.kind, parsedRoute.itemPath)
			: currentPage
				? pageKey(currentPage.path)
				: activeArtifact
					? artifactKey(activeArtifact.id)
					: undefined
	)
	let activeTabPickerOpen = $state(false)

	// Breadcrumb picks steer the *active* tab; the "+" picker opens new ones. An
	// editable item also becomes the session's live editor (owner.navigate).
	function navigatePreviewTo(target: PreviewTarget) {
		owner?.navigate(target)
	}

	// Names for the active session's item tabs, read from the same workspace
	// listing the pickers use (module-cached, so an opened picker makes this free).
	// The mounted editor's stamp is the live source, but it only fires for a tab
	// the user has visited — without this, restoring a session shows a path leaf on
	// every unvisited tab, each popping to its real name when first clicked.
	// Requested keys are tracked outside the state so filling the map can't re-run
	// the effect that fills it, and a key is released again on failure so a
	// transient network error doesn't strand every unvisited tab on its path leaf
	// for the lifetime of the page.
	const listedItemsRequested = new Set<string>()
	let listedItems = $state<Record<string, WorkspaceItem[]>>({})
	const listedKey = (workspace: string, kind: WorkspaceItemKind) => `${workspace}:${kind}`
	$effect(() => {
		const ws = previewWorkspace
		if (!ws) return
		for (const tab of owner?.tabs ?? []) {
			const route = parsePreviewItemRoute(tab.loc)
			if (!route) continue
			const key = listedKey(ws, route.kind)
			if (listedItemsRequested.has(key)) continue
			listedItemsRequested.add(key)
			void loadKind(ws, route.kind)
				.then((items) => {
					listedItems = { ...listedItems, [key]: items }
				})
				.catch((e) => {
					listedItemsRequested.delete(key)
					console.error(`Failed to load workspace ${route.kind}s`, e)
				})
		}
	})

	// Keyed by the tab's OWN session workspace, never the active one: every warm
	// session's tabs are labelled here, and two sessions on different forks can
	// hold the same item path.
	function listedItemFor(tab: SessionPreviewTab, workspace: string): WorkspaceItem | undefined {
		// A loaded editor supersedes the listing for good — including when it names
		// nothing, else clearing a summary would resurrect the listing's copy of it.
		if (tab.editorNamed) return undefined
		const route = parsePreviewItemRoute(tab.loc)
		if (!route) return undefined
		return listedItems[listedKey(workspace, route.kind)]?.find((i) => i.path === route.itemPath)
	}

	// Short tab label. An item whose live editor has loaded carries a
	// `friendlyLabel` that editor stamped — its summary, or the typed/auto name of
	// an item still parked at `…/draft_<uuid>` (the page can't read the runtime
	// cell reactively, so the editor mirrors the name onto the tab model). Before
	// that, the workspace listing names it. Falls back to the plain location label
	// for summary-less items and non-item pages.
	function tabLabelFor(tab: SessionPreviewTab, workspace: string): string {
		const listed = listedItemFor(tab, workspace)
		return (
			tab.friendlyLabel ??
			(listed && itemDisplayName(listed.path, listed.draftPath, listed.summary)) ??
			previewLocationLabel(tab.loc)
		)
	}

	// Hover title for a tab. A summary label is free text the strip truncates, and
	// it hides the path entirely, so the tooltip carries both. The path shown is
	// the item's staged one when it has one — a draft's `…/draft_<uuid>` storage
	// path names nothing to the reader.
	function tabTitleFor(tab: SessionPreviewTab, workspace: string): string {
		const label = tabLabelFor(tab, workspace)
		const path =
			tab.friendlyPath ??
			listedItemFor(tab, workspace)?.draftPath ??
			parsePreviewItemRoute(tab.loc)?.itemPath
		return path && path !== label ? `${label}\n${path}` : label
	}

	// A link click inside a live editor (e.g. a subflow reference) re-points the
	// active tab, which — for an editable item — makes it the session's live
	// editor via owner.navigate. Legacy drag-and-drop apps have no preview
	// wrapper, so they open in the standalone editor instead.
	function navigateEditorTo(item: WorkspaceItem) {
		if (item.kind === 'app' && !item.raw_app) {
			// Leaving the preview for the standalone editor — carry the session
			// workspace so the app opens in the fork the session acts on, not the
			// navigation workspace.
			goto(withWorkspaceParam(`${base}/apps/edit/${item.path}`, previewWorkspace))
			return
		}
		owner?.navigate({ type: 'item', item })
	}

	// A preview iframe that navigates to an editor route posts up to us instead of
	// booting the editor inside the frame (see the logged layout's beforeNavigate).
	// Retarget the active tab — the navigating frame is the visible one the user
	// just clicked in — which flips its seam from iframe → live editor.
	$effect(() => {
		function onMessage(e: MessageEvent) {
			if (e.origin !== window.location.origin) return
			const d = e.data
			if (!d) return
			// A preview frame navigating to an editor route: re-point the active tab to
			// the live in-process editor instead of booting a second one in the frame.
			if (d.type === 'wm.session.openEditor') {
				if (d.kind !== 'script' && d.kind !== 'flow' && d.kind !== 'raw_app') return
				if (typeof d.path !== 'string') return
				const item: WorkspaceItem =
					d.kind === 'raw_app'
						? { kind: 'app', raw_app: true, path: d.path, summary: '' }
						: { kind: d.kind, path: d.path, summary: '' }
				owner?.navigate({ type: 'item', item })
				return
			}
			// A job clicked inside a preview tab: open the run detail in a NEW tab so the
			// originating page (e.g. Runs) stays put. open() focuses an existing tab for
			// the same run rather than duplicating it.
			if (d.type === 'wm.session.openRun') {
				if (typeof d.href !== 'string') return
				owner?.open({
					type: 'page',
					href: d.href,
					label: typeof d.label === 'string' ? d.label : 'Run'
				})
				return
			}
		}
		window.addEventListener('message', onMessage)
		return () => window.removeEventListener('message', onMessage)
	})
</script>

<!-- A tab mutation inside the owner's debounce window would be lost to a
     reload/navigation; hidden fires before pagehide, so flush there. -->
<svelte:document
	onvisibilitychange={() => {
		if (document.visibilityState === 'hidden') owner?.flushNow()
	}}
/>

<div class="h-full flex flex-col min-h-0">
	{#if embedded}
		<!-- Rendered inside a preview iframe — opening the sessions UI here would
		     recurse. Offer to break out to the top-level window instead. -->
		<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
			<p class="text-primary font-medium">Sessions can't open inside a preview</p>
			<p>This page is being previewed in a session panel. Open it at the top level instead.</p>
			<Button
				size="xs"
				startIcon={{ icon: ExternalLink }}
				onclick={() => {
					const u = new URL(window.location.href)
					u.searchParams.delete('nomenubar')
					window.top?.location.assign(u.pathname + u.search)
				}}>Open sessions</Button
			>
		</div>
	{:else if $userStore?.operator}
		<!-- Operators are exempt from the sessions beta (the layout keeps their
		     legacy docked chat); a direct URL must not bypass that. -->
		<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
			<p class="text-primary font-medium">AI Sessions are not available for operators</p>
			<p>Use the Ask AI chat instead.</p>
			<Button
				size="xs"
				onclick={() => {
					try {
						localStorage.setItem('ai-chat-open', 'true')
					} catch {}
					window.location.href = `${base}/`
				}}
			>
				Open Ask AI chat
			</Button>
		</div>
	{:else if !globalEnabled}
		<!-- Direct navigation (bookmark, shared link) while the user has opted out
		     of the beta: offer the way back in instead of a dead end. -->
		<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
			<p class="text-primary font-medium">AI Sessions are deactivated</p>
			<p>You switched back to the legacy chat. Activate AI Sessions (beta) to open this page.</p>
			<Button size="xs" onclick={() => setSessionsBetaOptOut(false, `${base}/sessions`)}>
				Activate AI Sessions
			</Button>
		</div>
	{:else if !sessionState.hydrated}
		<!-- Sessions hydrate from IndexedDB after the user resolves; until then an
		     empty list means "loading", so recovery below must not fire and strand
		     the user in a new session while their own is still arriving. -->
		<div class="flex-1 flex items-center justify-center">
			<Loader2 class="animate-spin" />
		</div>
	{:else if !sessionName}
		<div class="p-8 text-secondary">No session selected — pick one in the sidebar.</div>
	{:else if !sessionByName || recovering}
		<!-- A tick while recovery swaps in an empty session, or the length of a
		     fork teardown's HTTP round trip while a delete holds recovery off. -->
		<div class="flex-1 flex items-center justify-center">
			<Loader2 class="animate-spin" />
		</div>
	{:else}
		<!-- The hidden-assistant verdict overlays the session stack rather than replacing
		     it: warm sessions and mounted preview editors must survive a cross-workspace
		     switch, and a verdict still loading would otherwise tear them down on every
		     one. The covered stack is inert so nothing under the overlay takes focus. -->
		<div class="flex-1 min-h-0 flex flex-col relative">
			<div
				class="flex-1 min-h-0 flex flex-row relative z-0"
				use:splitterPointerCapture
				inert={aiHiddenVerdict !== false}
			>
				<Splitpanes
					horizontal={false}
					class="flex-1 min-h-0 session-splitter {previewCollapsed ? 'splitter-off' : ''}"
				>
					{#if !fullscreen}
						<!-- Chat column. Warm sessions stay mounted (stacked, visibility-toggled)
					     so switching between them preserves chat scroll/draft state. -->
						<Pane bind:size={chatPaneSize} minSize={25} class="flex flex-col min-h-0">
							<div class="relative flex-1 min-h-0">
								{#each warmSessions as s (s.id)}
									<div
										class="absolute inset-0 flex flex-col {s.id === activeSession?.id
											? 'z-10 opacity-100 pointer-events-auto'
											: 'z-0 opacity-0 pointer-events-none'}"
										aria-hidden={s.id !== activeSession?.id}
									>
										<SessionWrapper sessionId={s.id} />
									</div>
								{/each}
							</div>
						</Pane>
					{/if}

					<!-- Preview panel: the live Windmill page, framed like the editor pane.
				     Always mounted (collapse resizes it to 0 — see previewPaneSize) so
				     warm sessions' preview hosts survive a collapsed active session. -->
					<Pane
						bind:size={previewPaneSize}
						minSize={previewCollapsed ? 0 : 30}
						maxSize={previewCollapsed ? 0 : 100}
						class="flex flex-col min-h-0"
					>
						<div class="flex-1 min-h-0 flex flex-col {fullscreen ? 'p-0' : 'p-2 pl-0'}">
							<div
								class="flex flex-col flex-1 min-h-0 overflow-hidden relative bg-surface {fullscreen
									? ''
									: 'rounded-md border border-light'}"
							>
								{#if !fullscreen}
									<!-- Collapse the preview panel — floats over the top-left corner so
									     the tab strip keeps the full width. -->
									<button
										type="button"
										onclick={() => owner?.setCollapsed(true)}
										title="Collapse preview"
										aria-label="Collapse preview"
										class="absolute top-1 left-1 z-30 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
									>
										<PanelRightClose size={14} />
									</button>
								{/if}

								<!-- Open-in-full-page + full-screen toggle, floating over the top-right
								     corner to mirror the collapse control. -->
								<div class="absolute top-1 right-1 z-30 flex items-center gap-0.5">
									{#if !activeTabIsArtifact}
										<a
											href={withWorkspaceParam(
												owner?.activeTab?.loc || owner?.activeTab?.url || `${base}/`,
												previewWorkspace
											)}
											title="Open in workspace"
											aria-label="Open in workspace"
											class="inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
										>
											<ExternalLink size={14} />
										</a>
									{/if}
									<button
										type="button"
										onclick={() => (fullscreen = !fullscreen)}
										title={fullscreen ? 'Exit full screen' : 'Full screen'}
										aria-label={fullscreen ? 'Exit full screen' : 'Full screen'}
										class="inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
									>
										{#if fullscreen}
											<Minimize2 size={14} />
										{:else}
											<Maximize2 size={14} />
										{/if}
									</button>
								</div>

								<!-- Tab strip: open preview pages, shared with the raw-app editor
								     (DraggableTabs). Clicking the active tab (label or accessory chevron)
								     toggles its breadcrumb picker; the "+" trailing opens the router picker.
								     Left/right padding clears the floating collapse/fullscreen buttons. -->
								<DraggableTabs
									tabs={previewTabItems}
									activeId={owner?.activeId ?? ''}
									onSelect={selectTab}
									onActiveClick={() => (activeTabPickerOpen = !activeTabPickerOpen)}
									onClose={closeTab}
									onReorder={reorderTabs}
									class="session-preview-tab-strip h-8 border-b border-light bg-surface-secondary/50 {fullscreen
										? 'pl-1.5'
										: 'pl-9'} pr-16"
								>
									{#snippet tabAccessory(_tab, isActive)}
										{#if isActive}
											<!-- Any active-tab click toggles the picker (`onActiveClick`); the tab
										     is excluded from pointerdown-outside so toggle doesn't race close.
										     The trigger is an inert whole-tab overlay (anchor only — clickable
										     would break dnd reorder); the chevron is purely visual. -->
											<Popover
												placement="bottom-start"
												usePointerDownOutside
												excludeSelectors=".drawer, .session-preview-tab-strip [role='tab'][aria-selected='true']"
												disableFocusTrap
												closeOnOtherPopoverOpen
												enableFlyTransition
												bind:isOpen={activeTabPickerOpen}
												openFocus="[data-workspace-picker-search]"
												contentClasses="flex flex-col overflow-hidden"
												class="absolute inset-0 pointer-events-none"
												triggerAttrs={{
													'aria-label': 'Change preview',
													tabindex: -1,
													// The inert trigger only ever receives focus from melt's
													// close-time restore; hand it straight to the tab so
													// arrow/Delete tab shortcuts keep working.
													onfocus: (e: FocusEvent) =>
														(e.currentTarget as HTMLElement)
															.closest<HTMLElement>('[role="tab"]')
															?.focus()
												}}
											>
												{#snippet content()}
													<!-- The picker snapshots its scope at mount, but `friendlyPath` is
												     stamped async once the editor cell loads — a picker opened
												     before the stamp is scoped to the `draft_<uuid>` storage
												     folder while the tree groups the draft under its friendly
												     folder. Remount on the scope dir so it re-lands on the item. -->
													{#key activePickerScope?.dir ?? ''}
														<PreviewRouterPicker
															initialScope={activePickerScope}
															initialHighlight={activePickerHighlight}
															{currentItem}
															workspaceId={previewWorkspace}
															artifacts={sessionArtifacts}
															onPick={(t) => {
																activeTabPickerOpen = false
																navigatePreviewTo(t)
															}}
														/>
													{/key}
												{/snippet}
											</Popover>
											<ChevronDown
												size={12}
												class="shrink-0 text-tertiary group-hover:text-primary"
											/>
										{/if}
									{/snippet}
									{#snippet afterTabs()}
										<Popover
											placement="bottom-start"
											usePointerDownOutside
											excludeSelectors=".drawer"
											disableFocusTrap
											closeOnOtherPopoverOpen
											bind:isOpen={newTabOpen}
											enableFlyTransition
											openFocus="[data-workspace-picker-search]"
											contentClasses="flex flex-col overflow-hidden"
											class="shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover cursor-pointer"
										>
											{#snippet trigger()}
												<Plus size={14} />
											{/snippet}
											{#snippet content()}
												<PreviewRouterPicker
													workspaceId={previewWorkspace}
													artifacts={sessionArtifacts}
													onPick={(t) => {
														newTabOpen = false
														openInNewTab(t)
													}}
												/>
											{/snippet}
										</Popover>
									{/snippet}
								</DraggableTabs>

								<!-- One host per tab of every warm session, stacked and
								     visibility-toggled so switching tabs or sessions never reloads
								     a mounted tab — hosts live as long as the session's runtime,
								     content-gated by the shared mount MRU. Each host renders a
								     live editor (script/flow/raw_app target) or an iframe fallback. -->
								<div class="relative flex-1 min-h-0">
									{#each warmSessions as s (s.id)}
										{@const rt = getRuntime(s.id)}
										{@const tabs = rt?.previewTabs}
										{#each tabs?.tabs ?? [] as tab (tab.id)}
											<!-- tabHosts is an imperative ref-bag (only tabHosts[key]?.reload() in
										     reloadTabs); it is intentionally a plain object so component
										     instances aren't proxied. Nothing reads it reactively, so the
										     non-reactive binding is fine. -->
											<!-- svelte-ignore binding_property_non_reactive -->
											<PreviewTabHost
												bind:this={tabHosts[tabKey(s.id, tab.id)]}
												{tab}
												session={s}
												runtime={rt}
												active={s.id === activeSession?.id && tab.id === tabs?.activeId}
												collapsed={(tabs?.collapsed ?? false) && !fullscreen}
												mounted={mountedTabKeys.has(tabKey(s.id, tab.id))}
												label={tabLabelFor(tab, s.workspace_id ?? '')}
												darkMode={isDarkMode.val}
												{fullscreen}
												onNavigate={navigateEditorTo}
												onLoad={(frame) => tabs && onTabLoad(tabs, tab, frame)}
											/>
										{/each}
									{/each}
									{#if (owner?.tabs.length ?? 0) === 0}
										<!-- New session with nothing to preview: an empty state with a
										     picker to open one, instead of defaulting to the home page. -->
										<div
											class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-center px-6 bg-surface"
										>
											<MonitorPlay size={28} class="text-tertiary" />
											<div class="flex flex-col gap-1">
												<span class="text-sm font-medium text-secondary">No preview open</span>
												<span class="text-xs text-tertiary max-w-xs"
													>Open a page, flow, script or app to preview it alongside the chat.</span
												>
											</div>
											<Popover
												placement="bottom"
												usePointerDownOutside
												excludeSelectors=".drawer"
												disableFocusTrap
												closeOnOtherPopoverOpen
												bind:isOpen={emptyStateNewTabOpen}
												enableFlyTransition
												openFocus="[data-workspace-picker-search]"
												contentClasses="flex flex-col overflow-hidden"
											>
												{#snippet trigger()}
													<span
														class="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-md text-xs border border-light text-secondary hover:bg-surface-hover cursor-pointer"
													>
														<Plus size={14} /> Open a preview
													</span>
												{/snippet}
												{#snippet content()}
													<PreviewRouterPicker
														workspaceId={previewWorkspace}
														artifacts={sessionArtifacts}
														onPick={(t) => {
															emptyStateNewTabOpen = false
															openInNewTab(t)
														}}
													/>
												{/snippet}
											</Popover>
										</div>
									{/if}
								</div>
							</div>
						</div>
					</Pane>
				</Splitpanes>
				{#if previewCollapsed && !fullscreen}
					<!-- Collapsed preview: no rail — a floating launcher in the top-right to
				     reopen the side panel. -->
					<div class="absolute top-2 right-3 z-50">
						<Button
							variant="subtle"
							unifiedSize="sm"
							startIcon={{ icon: PanelRightOpen }}
							title="Open side panel"
							onclick={() => owner?.setCollapsed(false)}
						>
							Open side panel
						</Button>
					</div>
				{/if}
			</div>
			{#if aiHiddenVerdict === undefined}
				<div class="absolute inset-0 z-20 flex items-center justify-center bg-surface">
					<Loader2 class="animate-spin" />
				</div>
			{:else if aiHiddenVerdict}
				<!-- The workspace hid the assistant, and the sidebar switch with it, so only a
				     direct URL or a session acting on such a workspace lands here. -->
				<div
					class="absolute inset-0 z-20 bg-surface p-8 flex flex-col items-start gap-3 text-secondary text-sm"
				>
					<p class="text-primary font-medium">AI Sessions are hidden in this workspace</p>
					<p>A workspace admin hid AI sessions in the workspace settings.</p>
					<Button unifiedSize="sm" onclick={() => goto('/')}>Back to workspace</Button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* Draggable gutter between the chat and the preview: a real (layout-occupying)
	   10px-wide grab zone, no overlap tricks that could cover the chat's scrollbar
	   or the preview's edge. Transparent at rest; on hover the app-global
	   `.splitpanes__splitter::after` grabber fades in. Uses a dedicated class, not
	   the shared `.splitter-hidden`, which force-zeroes splitter opacity and would
	   hide that grabber. */
	:global(.splitpanes--vertical.session-splitter) > :global(.splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		width: 10px !important;
	}
	/* Inset the global hover grabber from the pane's top/bottom edges so the line
	   doesn't run the full height, and round its ends into a pill — a lighter,
	   more contained hint. */
	:global(.splitpanes--vertical.session-splitter) > :global(.splitpanes__splitter)::after {
		top: 8px !important;
		bottom: 8px !important;
		height: auto !important;
		border-radius: 9999px !important;
	}

	/* Collapsed preview: the pane is resized to 0 but stays mounted, so remove
	   the gutter entirely — it would otherwise leave a dead 10px drag zone on the
	   chat's right edge. */
	:global(.splitpanes--vertical.splitter-off) > :global(.splitpanes__splitter) {
		display: none !important;
	}
</style>
