<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import {
		Plus,
		Maximize2,
		Minimize2,
		ExternalLink,
		PanelRightClose,
		PanelRightOpen,
		X
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { slide } from 'svelte/transition'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import PreviewRouterPicker from '$lib/components/sessions/PreviewRouterPicker.svelte'
	import { randomUUID } from '$lib/utils/uuid'
	import { goto } from '$lib/navigation'
	import SessionWrapper from '$lib/components/sessions/SessionWrapper.svelte'
	import {
		createSession,
		selectSession,
		sessionState,
		syncWorkspaceTo
	} from '$lib/components/sessions/sessionState.svelte'
	import {
		getOrCreateRuntime,
		getRuntime,
		listRuntimes,
		promoteEditorWarm
	} from '$lib/components/sessions/sessionRuntime.svelte'
	import { markSessionSeen } from '$lib/components/sessions/sessionUnread.svelte'
	import { sessionPreviewUrl, withMenuHidden } from '$lib/components/sessions/sessionMode.svelte'
	import { isGlobalAiEnabled } from '$lib/components/copilot/chat/global/gate'
	import { userWorkspaces, type UserWorkspace } from '$lib/stores'
	import { base } from '$lib/base'
	import PreviewRouterSegment from '$lib/components/sessions/PreviewRouterSegment.svelte'
	import {
		matchPreviewPage,
		pageKey,
		stripBase,
		type PreviewTarget
	} from '$lib/components/sessions/previewRouter'
	import {
		dirKey,
		editPathFor,
		KIND_LABEL_LOWER,
		kindKey,
		leafKeyFor,
		type WorkspaceItem,
		type WorkspaceItemKind
	} from '$lib/components/workspacePicker'

	const globalEnabled = isGlobalAiEnabled()

	const sessionName = $derived(page.url.searchParams.get('session_name') ?? '')

	// Unfiltered resolution by name — used to drive workspace switching
	// when a deep-linked session lives outside the current workspace.
	const sessionByName = $derived(
		sessionName ? sessionState.sessions.find((s) => s.name === sessionName) : undefined
	)

	// If the deep-linked session committed to a workspace different from
	// the active one, switch globally so visibility resolves and the chat
	// loads against the right workspace. Skip the switch when the target
	// workspace is no longer in the user's list — pointing the global
	// workspace at a deleted id would break sidebar scope.
	$effect(() => {
		const ws = sessionByName?.workspace_id
		if (!ws) return
		if (!$userWorkspaces.find((w) => w.id === ws)) return
		untrack(() => syncWorkspaceTo(ws))
	})

	// Resolve by name without applying the sidebar scope filter so an open
	// chat survives workspace switches.
	const activeSession = $derived(sessionState.sessions.find((s) => s.name === sessionName))

	// Touch the runtime for the active session so it gets created on first visit.
	// Also refresh the fork diff count: deep-link / back-button navigation
	// changes the URL but doesn't fire the picker's activate path. Gate on
	// session identity (id) rather than the full activeSession derived — the
	// sessions array mutates on every persisted change (including token-by-token
	// streaming), so a value-trigger would re-fetch compareWorkspaces dozens of
	// times per turn.
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
			const rt = getOrCreateRuntime(session)
			void rt.refreshForkComparison()
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
	// preserves each previewed page's scroll/edit state.
	const previewUrl = $derived(sessionPreviewUrl(activeSession))

	// Each tab tracks two URLs: `url` is what we *command* the iframe to load
	// (changes only on an explicit navigate — breadcrumb pick / new tab), `loc`
	// is the *observed* location reported back on load. Keeping them separate is
	// what lets a tab stay mounted: navigating inside the iframe updates `loc`
	// only, so the `src`-bound `url` doesn't change and the frame never reloads.
	type PreviewTab = { id: string; url: string; loc: string; pinned?: boolean }
	let tabs = $state<PreviewTab[]>([])
	let activeTabId = $state('session')
	const activeTab = $derived(tabs.find((t) => t.id === activeTabId) ?? tabs[0])
	// The first tab is pinned to the session's own view; reset on session change.
	$effect(() => {
		void activeSession?.id
		untrack(() => {
			tabs = [{ id: 'session', url: previewUrl, loc: previewUrl, pinned: true }]
			activeTabId = 'session'
		})
	})

	function targetUrl(target: PreviewTarget): string {
		return target.type === 'page' ? target.href : `${base}${editPathFor(target.item)}`
	}
	function selectTab(id: string) {
		activeTabId = id
	}
	function openInNewTab(target: PreviewTarget) {
		const id = randomUUID()
		const url = targetUrl(target)
		tabs.push({ id, url, loc: url })
		activeTabId = id
	}
	function closeTab(id: string) {
		const idx = tabs.findIndex((t) => t.id === id)
		if (idx < 0 || tabs[idx].pinned) return
		tabs.splice(idx, 1)
		if (activeTabId === id) activeTabId = (tabs[idx] ?? tabs[idx - 1] ?? tabs[0])?.id ?? 'session'
	}
	let newTabOpen = $state(false)

	let fullscreen = $state(false)
	// Collapse the preview panel to give the chat the full width.
	let previewCollapsed = $state(false)

	// Workspace breadcrumb shown in the preview header: the session's family · fork.
	function findSessionRoot(id: string | undefined): UserWorkspace | undefined {
		if (!id) return undefined
		let current = $userWorkspaces.find((w) => w.id === id)
		while (current?.parent_workspace_id) {
			const parent = $userWorkspaces.find((w) => w.id === current!.parent_workspace_id)
			if (!parent) break
			current = parent
		}
		return current
	}
	const sessionWsId = $derived(
		activeSession ? (activeSession.workspace_id ?? activeSession.pending_workspace_id) : undefined
	)
	const sessionWs = $derived(
		sessionWsId ? $userWorkspaces.find((w) => w.id === sessionWsId) : undefined
	)
	const sessionRoot = $derived(findSessionRoot(sessionWsId))
	const sessionFamilyName = $derived(sessionRoot?.name ?? sessionWs?.name ?? sessionWsId)
	const sessionIsFork = $derived(
		!!activeSession?.pending_fork ||
			(!!sessionWs && !!sessionRoot && sessionWs.id !== sessionRoot.id)
	)
	const sessionForkName = $derived(
		activeSession?.pending_fork?.name ?? (sessionIsFork ? sessionWs?.name : 'main')
	)

	// Page path shown after the workspace breadcrumb — the active tab's observed
	// location, so the breadcrumb tracks where the user browses inside the tab.
	const displayPath = $derived(activeTab?.loc ?? activeTab?.url ?? previewUrl)
	function onTabLoad(tab: PreviewTab, frame: HTMLIFrameElement) {
		try {
			const loc = frame.contentWindow?.location
			if (!loc) return
			// Drop the injected `nomenubar` flag so the breadcrumb stays readable.
			const u = new URL(loc.href)
			u.searchParams.delete('nomenubar')
			tab.loc = u.pathname + u.search
		} catch {
			// Best-effort: the preview is same-origin, but reading location could
			// still throw mid-navigation — keep the seeded path in that case.
		}
	}

	// Editor-style breadcrumb over the previewed page. We only render clickable
	// segments when the preview is sitting on a script/flow/app route — for any
	// other page (home, runs, …) there's no item to drill into, so we fall back
	// to the plain path.
	type ParsedRoute = { kind: WorkspaceItemKind; raw_app: boolean; itemPath: string }
	function parseItemRoute(fullPath: string): ParsedRoute | null {
		const p = stripBase(fullPath)
		const m = p.match(/^\/(scripts|flows|apps|apps_raw)\/(?:edit|get)\/(.+)$/)
		if (!m) return null
		const itemPath = decodeURIComponent(m[2])
		if (m[1] === 'scripts') return { kind: 'script', raw_app: false, itemPath }
		if (m[1] === 'flows') return { kind: 'flow', raw_app: false, itemPath }
		if (m[1] === 'apps_raw') return { kind: 'app', raw_app: true, itemPath }
		return { kind: 'app', raw_app: false, itemPath }
	}

	const parsedRoute = $derived(parseItemRoute(displayPath))

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

	// On a non-item page, identify the known workspace page so the fallback
	// segment shows its name (e.g. "Workspace settings") and the picker
	// highlights it.
	const currentPage = $derived(parsedRoute ? undefined : matchPreviewPage(displayPath))

	// Breadcrumb picks steer the *active* tab; the "+" picker opens new ones.
	// Set `loc` too so the breadcrumb updates immediately, before the reload.
	function navigatePreviewTo(target: PreviewTarget) {
		const t = tabs.find((x) => x.id === activeTabId)
		if (!t) return
		const url = targetUrl(target)
		t.url = url
		t.loc = url
	}

	// Short tab label: a known page's name, else the item's leaf name, else path.
	function tabLabel(url: string): string {
		const page = matchPreviewPage(url)
		if (page) return page.label
		const parsed = parseItemRoute(url)
		if (parsed) return parsed.itemPath.split('/').pop() ?? parsed.itemPath
		return stripBase(url)
	}
</script>

<div class="h-full flex flex-col min-h-0">
	{#if !globalEnabled}
		<div class="p-8 text-secondary text-sm">
			Sessions are gated on the global-AI dev flag. Enable with
			<code class="text-2xs font-mono">localStorage.setItem('wm_dev_global_ai', '1')</code> and reload.
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
		<div class="flex-1 min-h-0 flex flex-row relative">
			<Splitpanes horizontal={false} class="flex-1 min-h-0 splitter-hidden">
				{#if !fullscreen}
					<!-- Chat column. Warm sessions stay mounted (stacked, visibility-toggled)
					     so switching between them preserves chat scroll/draft state. -->
					<!-- No explicit `size`: Splitpanes auto-distributes — when the preview
				     pane unmounts on collapse the chat fills 100%; with both panes it
				     splits evenly. An explicit size would pin the chat and leave a gap. -->
					<Pane minSize={25} class="flex flex-col min-h-0">
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

				<!-- Preview panel: the live Windmill page, framed like the editor pane. -->
				{#if !previewCollapsed}
					<Pane minSize={30} class="flex flex-col min-h-0">
						<div class="flex-1 min-h-0 flex flex-col {fullscreen ? 'p-0' : 'p-2 pl-0'}">
							<div
								transition:slide={{ axis: 'x', duration: 200 }}
								class="flex flex-col flex-1 min-h-0 overflow-hidden relative bg-surface {fullscreen
									? ''
									: 'rounded-md border border-light'}"
							>
								<!-- Tab strip: open preview pages. The first tab is pinned to the
								     session's own view; "+" opens the router picker to add more. -->
								<div
									class="flex items-center gap-1 px-1.5 h-8 border-b border-light shrink-0 bg-surface-secondary overflow-x-auto"
								>
									{#each tabs as tab (tab.id)}
										<div
											class="group/tab flex items-center gap-1 shrink-0 max-w-[12rem] h-6 pl-2 pr-1 rounded-md text-xs border transition-colors {tab.id ===
											activeTabId
												? 'bg-surface text-primary border-light'
												: 'text-secondary border-transparent hover:bg-surface-hover'}"
										>
											<button
												type="button"
												class="flex items-center gap-1.5 min-w-0"
												onclick={() => selectTab(tab.id)}
												title={tabLabel(tab.loc)}
											>
												{#if tab.pinned}
													<span class="w-1.5 h-1.5 rounded-full bg-current opacity-50 shrink-0"
													></span>
												{/if}
												<span class="truncate">{tabLabel(tab.loc)}</span>
											</button>
											{#if !tab.pinned}
												<button
													type="button"
													onclick={() => closeTab(tab.id)}
													title="Close tab"
													aria-label="Close tab"
													class="shrink-0 inline-flex items-center justify-center w-4 h-4 rounded text-tertiary hover:text-primary hover:bg-surface-hover opacity-0 group-hover/tab:opacity-100"
												>
													<X size={11} />
												</button>
											{/if}
										</div>
									{/each}
									<Popover
										placement="bottom-start"
										usePointerDownOutside
										excludeSelectors=".drawer"
										disableFocusTrap
										closeOnOtherPopoverOpen
										bind:isOpen={newTabOpen}
										openFocus="[data-workspace-picker-search]"
										class="shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover cursor-pointer"
									>
										{#snippet trigger()}
											<Plus size={14} />
										{/snippet}
										{#snippet content()}
											<PreviewRouterPicker
												onPick={(t) => {
													newTabOpen = false
													openInNewTab(t)
												}}
											/>
										{/snippet}
									</Popover>
								</div>

								<!-- Workspace breadcrumb: the session's family · fork + active tab path. -->
								<div
									class="pl-1 pr-3 h-8 flex items-center gap-1.5 border-b border-light shrink-0 text-xs text-secondary"
								>
									{#if !fullscreen}
										<!-- Collapse the preview panel (top-left), matching the legacy
										     session editor's collapse control. -->
										<button
											type="button"
											onclick={() => (previewCollapsed = true)}
											title="Collapse preview"
											aria-label="Collapse preview"
											class="shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
										>
											<PanelRightClose size={14} />
										</button>
									{/if}
									<span class="shrink-0 whitespace-nowrap">
										<span class="text-primary font-medium">{sessionFamilyName}</span>
										<span class="text-tertiary px-1">·</span>
										<span class={sessionIsFork ? 'text-accent font-medium' : 'text-tertiary'}>
											{sessionForkName}
										</span>
									</span>
									{#if parsedRoute}
										<nav
											aria-label="Breadcrumb"
											class="flex items-center min-w-0 font-mono text-secondary"
										>
											<PreviewRouterSegment
												label={KIND_LABEL_LOWER[parsedRoute.kind]}
												initialScope={undefined}
												initialHighlight={kindKey(parsedRoute.kind)}
												isCurrent={!segments}
												{currentItem}
												onPick={navigatePreviewTo}
											/>
											{#if segments}
												{#each segments.dirs as dir, i (dir.fullPath)}
													{@const dKey = dirKey('all', dir.fullPath)}
													<PreviewRouterSegment
														label={dir.name}
														withChevron
														extraClass={i === 0 ? 'gap-0.5 min-w-0 max-w-[40%]' : 'gap-0.5 min-w-0'}
														initialScope={i === 0
															? { kind: 'all' }
															: { kind: 'all', dir: segments.dirs[i - 1].fullPath }}
														initialHighlight={dKey}
														{currentItem}
														onPick={navigatePreviewTo}
													/>
												{/each}
												{@const leafKey = leafKeyFor(parsedRoute.kind, segments.leaf.fullPath)}
												{@const leafParent = segments.dirs[segments.dirs.length - 1]?.fullPath}
												<PreviewRouterSegment
													label={segments.leaf.name}
													withChevron
													extraClass="gap-0.5 min-w-0"
													initialScope={leafParent
														? { kind: 'all', dir: leafParent }
														: { kind: 'all' }}
													initialHighlight={leafKey}
													isCurrent
													{currentItem}
													onPick={navigatePreviewTo}
												/>
											{/if}
										</nav>
									{:else}
										<!-- Non-item page (home, runs, settings, …): the segment is still a
										     full router so you can jump anywhere from here. -->
										<nav
											aria-label="Breadcrumb"
											class="flex items-center min-w-0 font-mono text-secondary"
										>
											<PreviewRouterSegment
												label={currentPage?.label ?? (displayPath || '/')}
												extraClass="min-w-0 truncate"
												initialHighlight={currentPage ? pageKey(currentPage.path) : undefined}
												isCurrent
												{currentItem}
												onPick={navigatePreviewTo}
											/>
										</nav>
									{/if}
									<a
										href={activeTab?.url ?? previewUrl}
										title="Open full page"
										aria-label="Open full page"
										class="ml-auto shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
									>
										<ExternalLink size={14} />
									</a>
									<button
										type="button"
										onclick={() => (fullscreen = !fullscreen)}
										title={fullscreen ? 'Exit full screen' : 'Full screen'}
										aria-label={fullscreen ? 'Exit full screen' : 'Full screen'}
										class="shrink-0 inline-flex items-center justify-center w-6 h-6 rounded text-tertiary hover:text-primary hover:bg-surface-hover"
									>
										{#if fullscreen}
											<Minimize2 size={14} />
										{:else}
											<Maximize2 size={14} />
										{/if}
									</button>
								</div>
								<!-- One iframe per tab, stacked and visibility-toggled so every
								     tab stays mounted (switching never reloads). -->
								<div class="relative flex-1 min-h-0">
									{#each tabs as tab (tab.id)}
										<iframe
											src={withMenuHidden(tab.url)}
											onload={(e) => onTabLoad(tab, e.currentTarget as HTMLIFrameElement)}
											title="Session preview: {tabLabel(tab.loc)}"
											class="absolute inset-0 w-full h-full border-0 bg-surface {tab.id ===
											activeTabId
												? 'z-10 opacity-100 pointer-events-auto'
												: 'z-0 opacity-0 pointer-events-none'}"
										></iframe>
									{/each}
								</div>
							</div>
						</div>
					</Pane>
				{/if}
			</Splitpanes>
			{#if previewCollapsed && !fullscreen}
				<!-- Collapsed preview: no rail — a floating launcher in the top-right to
				     reopen the side panel, mirroring the previous collapsed-rail launcher. -->
				<div class="absolute top-2 right-3 z-50">
					<Button
						variant="subtle"
						unifiedSize="sm"
						startIcon={{ icon: PanelRightOpen }}
						title="Open side panel"
						onclick={() => (previewCollapsed = false)}
					>
						Open side panel
					</Button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* Invisible-but-draggable splitter between the chat and the preview. */
	:global(.splitter-hidden .splitpanes__splitter) {
		background-color: transparent !important;
		border: none !important;
		opacity: 0 !important;
	}
</style>
