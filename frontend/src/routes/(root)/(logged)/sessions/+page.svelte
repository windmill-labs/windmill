<script lang="ts">
	import { untrack } from 'svelte'
	import { page } from '$app/state'
	import {
		Plus,
		Maximize2,
		Minimize2,
		ExternalLink,
		PanelRightClose,
		PanelRightOpen
	} from 'lucide-svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { slide } from 'svelte/transition'
	import { Button } from '$lib/components/common'
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
	import { userWorkspaces } from '$lib/stores'

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

	// Preview panel: iframe the active session's view (captured page / target).
	// `previewUrl` stays clean for the breadcrumb + "open full page" link;
	// `iframeSrc` hides the previewed page's own sidebar (the sessions page
	// already provides navigation).
	const previewUrl = $derived(sessionPreviewUrl(activeSession))
	const iframeSrc = $derived(withMenuHidden(previewUrl))
	let fullscreen = $state(false)
	// Collapse the preview panel to give the chat the full width.
	let previewCollapsed = $state(false)

	// Path shown in the breadcrumb. Seeded from the preview URL, then refreshed
	// from the iframe on navigation (same-origin) so it tracks where the user
	// browses inside the preview.
	let displayPath = $state('')
	$effect(() => {
		displayPath = previewUrl
	})
	let previewFrame: HTMLIFrameElement | undefined = $state()
	function onPreviewLoad() {
		try {
			const loc = previewFrame?.contentWindow?.location
			if (!loc) return
			// Drop the injected `nomenubar` flag so the breadcrumb stays readable.
			const u = new URL(loc.href)
			u.searchParams.delete('nomenubar')
			displayPath = u.pathname + u.search
		} catch {
			// Best-effort: the preview is same-origin, but reading location could
			// still throw mid-navigation — keep the seeded path in that case.
		}
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
								<!-- Breadcrumb: reflects what the preview iframe is showing. -->
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
									<span class="text-tertiary">Viewing</span>
									<span class="font-mono truncate">{displayPath}</span>
									<a
										href={previewUrl}
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
								<iframe
									bind:this={previewFrame}
									src={iframeSrc}
									onload={onPreviewLoad}
									title="Session preview"
									class="flex-1 min-h-0 w-full border-0 bg-surface"
								></iframe>
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
