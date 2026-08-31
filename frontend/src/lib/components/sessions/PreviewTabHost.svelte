<script lang="ts">
	import { untrack } from 'svelte'
	import { workspaceStore } from '$lib/stores'
	import { whereIs } from './sessionPreviewTabs.svelte'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import {
		getEffectiveWorkspaceId,
		sessionState,
		type Session,
		type SessionPreviewTab
	} from './sessionState.svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { Loader2 } from 'lucide-svelte'
	import {
		resolvePreviewTab,
		parsePreviewItemRoute,
		parsePreviewSelectedId,
		showsView
	} from './previewRouter'
	import { withMenuHidden } from './sessionMode.svelte'
	import ArtifactViewer from '../copilot/chat/artifacts/ArtifactViewer.svelte'
	import { setOverlayHost } from '../common/overlayHost.svelte'

	let {
		tab,
		session,
		runtime,
		active,
		collapsed = false,
		mounted,
		label,
		darkMode,
		fullscreen = false,
		onNavigate,
		onLoad
	}: {
		tab: SessionPreviewTab
		session: Session | undefined
		runtime: SessionRuntime | undefined
		/** Visible tab — only one is at a time; the rest stay mounted but hidden. */
		active: boolean
		/** Preview panel is not on screen — collapsed, and not overridden by full screen.
		 * The panel is never unmounted, so this is the difference between the active tab
		 * and a tab the user can actually see. Resolved by the page, which owns the
		 * collapse/full-screen precedence. */
		collapsed?: boolean
		/** Preview panel is in full screen — forwarded to editor views so a script
		 * editor reopens its test pane when there's room. */
		fullscreen?: boolean
		/** Lazy-mount gate: content only renders once the tab has been activated. */
		mounted: boolean
		/** Short tab label, for the iframe title. */
		label: string
		/** Current top-document theme — mirrored into page iframes so they follow
		 * live toggles (app iframes pin their own theme and are excluded). */
		darkMode: boolean
		/** A link click inside a live editor re-points the active preview tab. */
		onNavigate: (item: WorkspaceItem) => void
		/** Iframe finished loading — the page reads back its observed location. */
		onLoad: (frame: HTMLIFrameElement) => void
	} = $props()

	// Editor vs iframe is decided purely from the tab URL (see resolvePreviewTab):
	// any editable item (script/flow/raw app) or a pipeline folder mounts its own
	// live editor.
	const slot = $derived(resolvePreviewTab(tab.url))
	// Where inside the editor the tab was opened on ("open this flow step in a
	// session"). Only the in-process editors need it handed over — an iframe tab
	// loads the URL whole, params included.
	const selectedId = $derived(parsePreviewSelectedId(tab.url))
	const workspaceId = $derived(
		session ? (getEffectiveWorkspaceId(session) ?? $workspaceStore ?? '') : ''
	)
	const isActiveSession = $derived(!!session && sessionState.currentSessionId === session.id)

	// Resolved live from the session's store so an update_artifact re-renders the panel.
	const artifact = $derived(
		slot.kind === 'artifact'
			? runtime?.manager.artifacts.artifacts.find((a) => a.id === slot.id)
			: undefined
	)

	let frame: HTMLIFrameElement | undefined = $state()

	// Pages whose theme we mirror on live toggles. Regular apps are the only item
	// route that resolves to an iframe (scripts/flows/raw apps mount live editors)
	// and they pin their own theme, so excluding item routes excludes exactly them.
	const isPageIframe = $derived(slot.kind === 'iframe' && parsePreviewItemRoute(tab.url) === null)

	function applyPageIframeTheme(dark: boolean, target: HTMLIFrameElement | undefined = frame) {
		if (!isPageIframe) return
		try {
			target?.contentWindow?.document?.documentElement.classList.toggle('dark', dark)
		} catch {
			// Mid-navigation (or a defensively cross-origin frame); the next load re-applies.
		}
	}

	// Only live toggles need this; initial paint is already correct — the iframe's
	// own layout reads the global preference at load.
	$effect(() => {
		applyPageIframeTheme(darkMode)
	})

	export function reload() {
		// A live editor shares the runtime store the chat mutates, so generic chat
		// edits are already reflected — no reload needed. Deploys refresh it via
		// each editor view's onDeploy → runtime.syncPreviewWithDeployed. So only the
		// iframe fallback (a separate page) has to be told to refresh.
		if (slot.kind === 'editor') return
		try {
			const win = frame?.contentWindow
			if (!win) return
			// Reload the page the user is actually viewing (observed `loc`, canonical
			// with nomenubar/workspace stripped), re-injecting nomenubar + workspace.
			// A plain location.reload() would reload the iframe's current URL, which
			// in-frame navigation may have stripped of ?workspace= — booting the frame
			// into the top-level navigation workspace instead of the session fork
			// (sessionStorage/localStorage are shared with the top window, so the
			// scoping can only live in the URL). But replace() to the frame's exact
			// current URL is a no-op when it carries a fragment (same-document
			// navigation, no load) — only then fall back to location.reload(), which
			// always performs a full load of that same URL.
			const target = withMenuHidden(whereIs(tab), workspaceId || undefined)
			const { pathname, search, hash } = win.location
			if (pathname + search + hash === target) win.location.reload()
			else if (pathname + search === target.split('#')[0]) {
				// Only the fragment differs: replace() would navigate within the same
				// document, so the page never re-runs the hash handling that opens a
				// drawer. Land on the target, then force the load.
				win.location.replace(target)
				win.location.reload()
			} else win.location.replace(target)
		} catch {
			// Cross-navigation timing — skip; the next mutation reloads again.
		}
	}

	const visibility = $derived(
		active ? 'z-10 opacity-100 pointer-events-auto' : 'z-0 opacity-0 pointer-events-none'
	)

	// Overlays a tab opens (drawers, modals, popovers) anchor here rather than to the
	// document, so they stay within this tab and hide with it when another tab takes over.
	// Every branch that renders content in-realm must bind this — an unbound host makes the
	// overlay fall back to viewport-`fixed`, spilling it across the whole app.
	// The stack is per-tab for the same reason: this host stays mounted while hidden, and a
	// shared stack would let its overlays arbitrate Escape for the tab the user is looking at.
	let overlayHostEl: HTMLDivElement | undefined = $state()
	let hostDrawers = $state({ val: [] as string[] })
	setOverlayHost({
		el: () => overlayHostEl,
		drawers: hostDrawers,
		// The panel is resized rather than unmounted, so the active tab of a panel that
		// is off screen is as invisible as a background tab.
		active: () => active && !collapsed
	})

	let flashing = $state(false)
	let flashTimer: ReturnType<typeof setTimeout> | undefined
	// Guard against the effect's non-pulse reruns (tab/runtime changes) firing a
	// flash. Seeded from the current nonce: a pulse from before this host mounted
	// is moot, the tab appearing is itself the change the flash would point at.
	let lastPulseNonce = runtime?.previewTabs.focusPulse.nonce ?? -1
	$effect(() => {
		const pulse = runtime?.previewTabs.focusPulse
		if (!pulse || pulse.nonce === lastPulseNonce) return
		lastPulseNonce = pulse.nonce
		if (pulse.id !== tab.id) return
		flashing = true
		clearTimeout(flashTimer)
		flashTimer = setTimeout(() => (flashing = false), 800)
	})
	$effect(() => () => clearTimeout(flashTimer))

	// Where a booting frame starts, from the observed location: a tab remounted after the
	// user moved inside it should come back where they were, not where it started. Only
	// written while there is no frame — this host outlives the iframe (eviction keeps the
	// component, `mounted` gates only the markup below), so a value captured once would
	// send a remount back to whatever the tab held when it was opened.
	let bootSrc = $state(untrack(() => withMenuHidden(whereIs(tab), workspaceId || undefined)))

	// A live frame is navigated instead, and only when it is not already there. Binding
	// `src` reactively would navigate on every write to `tab.url` — including the anchor
	// drop that follows the user closing a drawer, where the frame already shows the
	// target and a fragment removal is a full load, not a same-document move.
	let lastCommanded = untrack(() => withMenuHidden(tab.url, workspaceId || undefined))
	// The workspace the frame was last sent to. A session re-scopes — switching workspace
	// before its first send, or a staged fork becoming the committed one — and the scope
	// lives in the URL alone, while `showsView` reads it as the noise it is for a location's
	// meaning. Tracked apart so a re-scope always reaches the frame.
	let lastScope = untrack(() => workspaceId)
	$effect(() => {
		const target = withMenuHidden(tab.url, workspaceId || undefined)
		const scope = workspaceId
		const live = mounted
		untrack(() => {
			const win = live ? frame?.contentWindow : undefined
			if (!win) {
				bootSrc = withMenuHidden(whereIs(tab), workspaceId || undefined)
				lastCommanded = target
				lastScope = scope
				return
			}
			if (target === lastCommanded) return
			const rescoped = scope !== lastScope
			// A frame the user browsed to another origin refuses the read but not the
			// navigation, and navigating it is the only way back to a Windmill page. So the
			// command counts as applied once acted on, never before.
			let here: string | undefined
			try {
				const { pathname, search, hash } = win.location
				here = pathname + search + hash
			} catch {
				here = undefined
			}
			// By view, not by string: a page hands its params back in an order and encoding
			// of its own, and re-loading the frame over that costs the user their scroll
			// position and everything else it holds outside the URL.
			if (!rescoped && here !== undefined && (here === target || showsView(here, target))) {
				lastCommanded = target
				return
			}
			try {
				win.location.replace(target)
				lastCommanded = target
				lastScope = scope
			} catch {
				// Cross-navigation timing — the next command navigates again.
			}
		})
	})

	// Forced-load signal for a navigation to the tab's exact current URL (see
	// pulseReload) — without it the page never re-runs its URL-driven behavior.
	// Seeded from the current nonce: a pulse from before this host mounted is
	// already satisfied by the initial iframe load.
	let lastReloadNonce = runtime?.previewTabs.reloadPulse.nonce ?? -1
	$effect(() => {
		const pulse = runtime?.previewTabs.reloadPulse
		if (!pulse || pulse.nonce === lastReloadNonce) return
		lastReloadNonce = pulse.nonce
		if (pulse.id !== tab.id) return
		reload()
	})
</script>

{#snippet editorLoading()}
	<div class="flex-1 flex items-center justify-center text-tertiary">
		<Loader2 class="animate-spin" />
	</div>
{/snippet}

{#if slot.kind === 'editor' && mounted && runtime}
	<div
		bind:this={overlayHostEl}
		class="absolute inset-0 flex flex-col min-h-0 bg-surface {visibility}"
		aria-hidden={!active}
	>
		<!-- Dynamic imports: the live editors pull in the heaviest module graphs in
		     the app (FlowBuilder, ScriptBuilder/Monaco, the raw-app editor, the
		     pipeline graph). Loading them only when an editor tab first mounts keeps
		     the /sessions route chunk thin, so entering session mode stays snappy. -->
		{#if slot.editorKind === 'flow'}
			{#await import('./FlowEditorView.svelte')}
				{@render editorLoading()}
			{:then Module}
				<Module.default
					{runtime}
					path={slot.path}
					{workspaceId}
					{onNavigate}
					{isActiveSession}
					{active}
					initialSelectedId={selectedId}
				/>
			{/await}
		{:else if slot.editorKind === 'script'}
			{#await import('./ScriptEditorView.svelte')}
				{@render editorLoading()}
			{:then Module}
				<Module.default
					{runtime}
					path={slot.path}
					{workspaceId}
					{onNavigate}
					{isActiveSession}
					{active}
					{fullscreen}
				/>
			{/await}
		{:else if slot.editorKind === 'pipeline'}
			{#await import('./PipelineEditorView.svelte')}
				{@render editorLoading()}
			{:then Module}
				<Module.default {runtime} path={slot.path} {workspaceId} {isActiveSession} {active} />
			{/await}
		{:else}
			{#await import('./RawAppEditorView.svelte')}
				{@render editorLoading()}
			{:then Module}
				<Module.default
					{runtime}
					path={slot.path}
					{workspaceId}
					{onNavigate}
					{isActiveSession}
					{active}
				/>
			{/await}
		{/if}
	</div>
{:else if slot.kind === 'artifact' && mounted}
	<div
		bind:this={overlayHostEl}
		class="absolute inset-0 flex flex-col min-h-0 bg-surface {visibility}"
		aria-hidden={!active}
	>
		{#if artifact && runtime}
			<ArtifactViewer
				{artifact}
				store={runtime.manager.artifacts}
				pinned={slot.version}
				onPin={(version) => runtime?.previewTabs.pinArtifactVersion(artifact.id, version)}
			/>
		{:else if !runtime?.manager.artifacts.loading}
			<div class="p-4 text-sm text-tertiary">This artifact is no longer available.</div>
		{/if}
	</div>
{:else if mounted}
	<iframe
		bind:this={frame}
		src={bootSrc}
		onload={(e) => {
			const f = e.currentTarget as HTMLIFrameElement
			// Re-apply after load so a toggle that happened while the frame was
			// loading (its layout read the pre-toggle preference) isn't lost.
			applyPageIframeTheme(darkMode, f)
			onLoad(f)
		}}
		title="Session preview: {label}"
		class="absolute inset-0 w-full h-full border-0 bg-surface {visibility}"
	></iframe>
{/if}

<!-- Flash ring, a sibling of every tab body rather than a child of one: an
     editor's own opaque background (or an iframe's document) would paint over a
     ring drawn inside it. -->
<div
	class="pointer-events-none absolute inset-0 z-30 ring-2 ring-inset ring-border-accent transition-opacity duration-300 {flashing &&
	active
		? 'opacity-100'
		: 'opacity-0'}"
	aria-hidden="true"
></div>
