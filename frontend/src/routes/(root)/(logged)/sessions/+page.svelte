<script lang="ts">
	import { untrack } from 'svelte'
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
		Loader2,
		X
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PreviewRouterPicker, {
		type Scope
	} from '$lib/components/sessions/PreviewRouterPicker.svelte'
	import { goto } from '$lib/navigation'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import PreviewTabHost from '$lib/components/sessions/PreviewTabHost.svelte'
	import {
		createSession,
		getEffectiveWorkspaceId,
		selectSession,
		sessionInCurrentFamily,
		sessionState,
		type SessionPreviewTab
	} from '$lib/components/sessions/sessionState.svelte'
	import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
	import { enterSessionMode } from '$lib/components/sessions/sessionSwitch.svelte'
	import type { SessionPreviewTabs } from '$lib/components/sessions/sessionPreviewTabs.svelte'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import {
		getOrCreateRuntime,
		getRuntime,
		listRuntimes,
		promoteEditorWarm
	} from '$lib/components/sessions/sessionRuntime.svelte'
	import { markSessionSeen } from '$lib/components/sessions/sessionUnread.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { setToolCompletionListener } from '$lib/components/copilot/chat/shared'
	import { base } from '$lib/base'
	import {
		matchPreviewPage,
		pageKey,
		stripBase,
		parsePreviewItemRoute,
		type PreviewTarget
	} from '$lib/components/sessions/previewRouter'
	import { leafKeyFor, type WorkspaceItem } from '$lib/components/workspacePicker'
	import { splitterPointerCapture } from '$lib/utils/splitterPointerCapture'

	const globalEnabled = isGlobalAiEnabled()

	// The sessions page hosts preview iframes that load Windmill pages. If one of
	// those iframes navigates back to /sessions, mounting the full UI again would
	// nest another sessions page (with its own iframes) endlessly. Detect the
	// iframe context and refuse to mount when embedded.
	const embedded = typeof window !== 'undefined' && window.self !== window.top

	const sessionName = $derived(page.url.searchParams.get('session_name') ?? '')

	// Unfiltered resolution by name — drives the "session not found" fallback and
	// the active-session lookup below.
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
	// fresh one. The `session_name`-without-a-session case is left to the
	// not-found UI below.
	$effect(() => {
		if (embedded || !sessionState.hydrated) return
		// sessionInCurrentFamily reads these via get(), so track them explicitly.
		$workspaceStore
		$userWorkspaces
		const current = activeSession
		const shouldReenter = current ? !sessionInCurrentFamily(current) : !sessionName
		if (!shouldReenter) return
		untrack(() => void enterSessionMode({ replace: true }))
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

	// Promote the active session in the LRU. Mutations untracked so the effect
	// only re-runs when activeSession changes, not on its own writes.
	$effect(() => {
		const id = activeSession?.id
		if (!id) return
		untrack(() => promoteEditorWarm(id))
	})

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

	async function startNewSession() {
		const fresh = createSession()
		await goto(`/sessions?session_name=${encodeURIComponent(fresh.name)}`)
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
	// tabs boot lazily on first visible activation.
	$effect(() => {
		const sid = activeRuntime?.sessionId
		const activeId = owner?.activeId
		if (!sid || !activeId) return
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
	}
	let newTabOpen = $state(false)
	// Separate open flag for the empty-state launcher: it can be mounted at the
	// same time as the tab-strip "+" popover, so sharing one flag would open both
	// pickers at once.
	let emptyStateNewTabOpen = $state(false)

	let fullscreen = $state(false)
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
	let lastExpandedPreviewSize = 50
	$effect(() => {
		const collapsed = previewCollapsed
		const full = fullscreen
		untrack(() => {
			if (full) {
				// Chat pane is unmounted: the preview is the only pane and must own
				// the full width, not its remembered split share.
				previewPaneSize = 100
			} else if (collapsed) {
				if (previewPaneSize && previewPaneSize > 0 && previewPaneSize < 100) {
					lastExpandedPreviewSize = previewPaneSize
				}
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

	// Page path shown after the workspace breadcrumb — the active tab's observed
	// location, so the breadcrumb tracks where the user browses inside the tab.
	const displayPath = $derived(owner?.activeTab?.loc ?? owner?.activeTab?.url ?? `${base}/`)
	// Writes to the tab's own session model: a hidden warm session's iframe can
	// finish loading while another session is shown, and its location must not
	// land on the visible session's tabs.
	function onTabLoad(tabs: SessionPreviewTabs, tab: SessionPreviewTab, frame: HTMLIFrameElement) {
		try {
			const loc = frame.contentWindow?.location
			if (!loc) return
			// observeLocation canonicalizes away the injected nomenubar/workspace
			// params so the tab's `loc` stays symmetric with `url` for dedupe/display.
			tabs.observeLocation(tab.id, loc.pathname + loc.search)
		} catch {
			// Best-effort: the preview is same-origin, but reading location could
			// still throw mid-navigation — keep the seeded path in that case.
		}
	}

	// Reload mounted preview tabs affected by a mutating chat tool (write_/patch_/
	// delete_/deploy_/…; read/test/navigate tools don't match). Scoped to the changed
	// item so editing one item never blank-reboots an unrelated item's preview iframe
	// (a full-page /apps_raw/edit reload is jarring).
	const tabHosts: Record<string, PreviewTabHost | undefined> = {}
	const MUTATING_TOOL_RE = /^(write_|patch_|delete_|deploy_|discard_|set_|create_|update_|remove_)/
	let reloadHandle: ReturnType<typeof setTimeout> | undefined
	// Drained each flush: item paths touched since the last flush, and a flag for an
	// unresolved mutation that forces a full reload (safe fallback).
	let pendingReloadPaths = new Set<string>()
	let pendingReloadAll = false

	// Reload the batched-mutation tabs across all warm sessions' mounted tabs (a
	// hidden preview would otherwise show pre-mutation content on return). `null`
	// reloads all; otherwise an item-route iframe reloads only when its item was
	// touched. Non-item pages always reload; a live-editor slot no-ops in reload().
	function reloadTabs(paths: Set<string> | null) {
		for (const s of warmSessions) {
			const tabs = getRuntime(s.id)?.previewTabs?.tabs ?? []
			for (const tab of tabs) {
				const key = tabKey(s.id, tab.id)
				if (!mountedTabKeys.has(key)) continue
				if (paths) {
					const route = parsePreviewItemRoute(tab.url)
					if (route && !paths.has(route.itemPath)) continue
				}
				tabHosts[key]?.reload()
			}
		}
	}
	function flushReload() {
		const paths = pendingReloadAll ? null : pendingReloadPaths
		pendingReloadPaths = new Set()
		pendingReloadAll = false
		reloadTabs(paths)
	}
	$effect(() => {
		// Debounced so a burst of writes (the AI editing several files) reloads once.
		setToolCompletionListener((name, args) => {
			if (!MUTATING_TOOL_RE.test(name)) return
			// A workspace item path scopes the reload to that item. The raw-app file
			// tools (write_app_file, …) pass a leading-'/' frontend file path and edit
			// the active session's target app, so scope to the target. Anything else is
			// unresolved → reload everything (safe fallback).
			const p = typeof args?.path === 'string' ? args.path : undefined
			if (p && !p.startsWith('/')) pendingReloadPaths.add(p)
			else if (p && activeSession?.target?.path) pendingReloadPaths.add(activeSession.target.path)
			else pendingReloadAll = true
			clearTimeout(reloadHandle)
			reloadHandle = setTimeout(flushReload, 500)
		})
		return () => {
			clearTimeout(reloadHandle)
			pendingReloadPaths = new Set()
			pendingReloadAll = false
			setToolCompletionListener(undefined)
		}
	})

	// Editor-style breadcrumb over the previewed page. We only render clickable
	// segments when the preview is sitting on a script/flow/app route — for any
	// other page (home, runs, …) there's no item to drill into, so we fall back
	// to the plain path.
	const parsedRoute = $derived(parsePreviewItemRoute(displayPath))

	// Split the item path into breadcrumb dirs + leaf, mirroring EditorHeader:
	// scope (`f/<folder>` | `u/<user>`) → subfolders → item name.
	const segments = $derived.by(() => {
		const itemPath = parsedRoute?.itemPath
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
				: undefined
	)
	let activeTabPickerOpen = $state(false)

	// Breadcrumb picks steer the *active* tab; the "+" picker opens new ones. An
	// editable item also becomes the session's live editor (owner.navigate).
	function navigatePreviewTo(target: PreviewTarget) {
		owner?.navigate(target)
	}

	// Short tab label: a known page's name, else a run detail, else the item's leaf
	// name, else path.
	function tabLabel(url: string): string {
		const page = matchPreviewPage(url)
		if (page) return page.label
		const run = stripBase(url).match(/^\/run\/([^/?#]+)/)
		if (run) return `Run ${decodeURIComponent(run[1]).slice(0, 8)}`
		const parsed = parsePreviewItemRoute(url)
		if (parsed) return parsed.itemPath.split('/').pop() ?? parsed.itemPath
		return stripBase(url)
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
	{:else if !globalEnabled}
		<div class="p-8 text-secondary text-sm">
			Sessions are gated on the global-AI dev flag. Enable with
			<code class="text-2xs font-mono">localStorage.setItem('wm_dev_global_ai', '1')</code> and reload.
		</div>
	{:else if !sessionState.hydrated}
		<!-- Sessions hydrate from IndexedDB after the user resolves; until then an
		     empty list means "loading", so the not-found branch below must not fire. -->
		<div class="flex-1 flex items-center justify-center">
			<Loader2 class="animate-spin" />
		</div>
	{:else if !sessionName}
		<div class="p-8 text-secondary">No session selected — pick one in the sidebar.</div>
	{:else if !sessionByName}
		<!-- A session_name is in the URL but no session by that name exists — e.g. a
		     deleted session or a link opened in a different browser. -->
		<div class="p-8 flex flex-col items-start gap-3 text-secondary text-sm">
			<div class="flex flex-col gap-1">
				<p class="text-primary font-medium">Session not found</p>
				<p>
					No session named <code class="font-mono text-2xs">{sessionName}</code> exists. It may have
					been deleted, or this link was created in a different browser.
				</p>
			</div>
			<Button size="xs" startIcon={{ icon: Plus }} onclick={startNewSession}>New session</Button>
		</div>
	{:else}
		<div class="flex-1 min-h-0 flex flex-row relative" use:splitterPointerCapture>
			<Splitpanes
				horizontal={false}
				class="flex-1 min-h-0 splitter-hidden {previewCollapsed ? 'splitter-off' : ''}"
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
									<SessionWrapper sessionId={s.id} hideEditor />
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
									class="absolute top-1 left-1 z-30 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover bg-surface-secondary"
								>
									<PanelRightClose size={14} />
								</button>
							{/if}

							<!-- Open-in-full-page + full-screen toggle, floating over the top-right
								     corner to mirror the collapse control. -->
							<div class="absolute top-1 right-1 z-30 flex items-center gap-0.5">
								<a
									href={withWorkspaceParam(
										owner?.activeTab?.loc || owner?.activeTab?.url || `${base}/`,
										previewWorkspace
									)}
									title="Open in workspace"
									aria-label="Open in workspace"
									class="inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover bg-surface-secondary"
								>
									<ExternalLink size={14} />
								</a>
								<button
									type="button"
									onclick={() => (fullscreen = !fullscreen)}
									title={fullscreen ? 'Exit full screen' : 'Full screen'}
									aria-label={fullscreen ? 'Exit full screen' : 'Full screen'}
									class="inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover bg-surface-secondary"
								>
									{#if fullscreen}
										<Minimize2 size={14} />
									{:else}
										<Maximize2 size={14} />
									{/if}
								</button>
							</div>

							<!-- Tab strip: open preview pages. "+" opens the router picker to
								     add more. -->
							<div
								class="flex items-center gap-1 h-8 border-b border-light shrink-0 bg-surface-secondary overflow-x-auto {fullscreen
									? 'pl-1.5'
									: 'pl-9'} pr-16"
							>
								{#each owner?.tabs ?? [] as tab (tab.id)}
									<div
										class="group/tab flex items-center gap-1 shrink-0 max-w-[14rem] h-6 pl-2 pr-1 rounded-md text-xs border transition-colors {tab.id ===
										owner?.activeId
											? 'bg-surface text-primary border-light'
											: 'text-secondary border-transparent hover:bg-surface-hover'}"
									>
										{#if tab.id === owner?.activeId}
											<!-- Active tab doubles as its own breadcrumb picker. -->
											<Popover
												placement="bottom-start"
												usePointerDownOutside
												excludeSelectors=".drawer"
												disableFocusTrap
												closeOnOtherPopoverOpen
												enableFlyTransition
												bind:isOpen={activeTabPickerOpen}
												openFocus="[data-workspace-picker-search]"
												class="flex items-center gap-1.5 min-w-0 cursor-pointer"
											>
												{#snippet trigger()}
													<span class="truncate">{tabLabel(tab.loc)}</span>
													<ChevronDown size={12} class="shrink-0 text-tertiary" />
												{/snippet}
												{#snippet content()}
													<PreviewRouterPicker
														initialScope={activePickerScope}
														initialHighlight={activePickerHighlight}
														{currentItem}
														workspaceId={previewWorkspace}
														onPick={(t) => {
															activeTabPickerOpen = false
															navigatePreviewTo(t)
														}}
													/>
												{/snippet}
											</Popover>
										{:else}
											<button
												type="button"
												class="flex items-center gap-1.5 min-w-0"
												onclick={() => selectTab(tab.id)}
												title={tabLabel(tab.loc)}
											>
												<span class="truncate">{tabLabel(tab.loc)}</span>
											</button>
										{/if}
										<button
											type="button"
											onclick={() => closeTab(tab.id)}
											title="Close tab"
											aria-label="Close tab"
											class="shrink-0 inline-flex items-center justify-center w-4 h-4 rounded text-tertiary hover:text-primary hover:bg-surface-hover opacity-0 group-hover/tab:opacity-100"
										>
											<X size={11} />
										</button>
									</div>
								{/each}
								<Popover
									placement="bottom-start"
									usePointerDownOutside
									excludeSelectors=".drawer"
									disableFocusTrap
									closeOnOtherPopoverOpen
									bind:isOpen={newTabOpen}
									enableFlyTransition
									openFocus="[data-workspace-picker-search]"
									class="shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover cursor-pointer"
								>
									{#snippet trigger()}
										<Plus size={14} />
									{/snippet}
									{#snippet content()}
										<PreviewRouterPicker
											workspaceId={previewWorkspace}
											onPick={(t) => {
												newTabOpen = false
												openInNewTab(t)
											}}
										/>
									{/snippet}
								</Popover>
							</div>

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
										<PreviewTabHost
											bind:this={tabHosts[tabKey(s.id, tab.id)]}
											{tab}
											session={s}
											runtime={rt}
											active={s.id === activeSession?.id && tab.id === tabs?.activeId}
											mounted={mountedTabKeys.has(tabKey(s.id, tab.id))}
											label={tabLabel(tab.loc)}
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
	{/if}
</div>

<style>
	/* Invisible-but-draggable splitter between the chat and the preview: a real
	   (layout-occupying) gutter, wide enough to grab. No overlap tricks — the
	   zone can't cover the chat's scrollbar or the preview's edge. */
	:global(.splitpanes--vertical.splitter-hidden) > :global(.splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
		width: 10px !important;
	}

	/* Collapsed preview: the pane is resized to 0 but stays mounted, so remove
	   the (invisible) gutter entirely — it would otherwise leave a dead 10px
	   drag zone on the chat's right edge. */
	:global(.splitpanes--vertical.splitter-off) > :global(.splitpanes__splitter) {
		display: none !important;
	}
</style>
