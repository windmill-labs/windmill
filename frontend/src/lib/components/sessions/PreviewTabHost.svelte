<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import type { WorkspaceItem } from '$lib/components/workspacePicker'
	import {
		getEffectiveWorkspaceId,
		sessionState,
		type Session,
		type SessionPreviewTab
	} from './sessionState.svelte'
	import type { SessionRuntime } from './sessionRuntime.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { resolvePreviewTab, parsePreviewItemRoute } from './previewRouter'
	import { withMenuHidden } from './sessionMode.svelte'
	import ArtifactViewer from '../copilot/chat/artifacts/ArtifactViewer.svelte'

	let {
		tab,
		session,
		runtime,
		active,
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
		armFocusReclaim()
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
			const target = withMenuHidden(tab.loc || tab.url, workspaceId || undefined)
			const { pathname, search, hash } = win.location
			if (pathname + search + hash === target) win.location.reload()
			else win.location.replace(target)
		} catch {
			// Cross-navigation timing — skip; the next mutation reloads again.
		}
	}

	const iframeSrc = $derived(withMenuHidden(tab.url, workspaceId || undefined))

	// A preview frame boots the whole app, and SvelteKit's router focuses the
	// frame's document body on every navigation — which drags focus off whatever
	// the user is typing in (the chat composer, mid-message, while the assistant
	// opens or reloads previews). Hand it back, bounded to the load the session
	// itself triggered: a click into the frame is the user's own move to make.
	const FOCUS_RECLAIM_WINDOW_MS = 10_000
	// The frame's document has to be hooked before its app hydrates — which is well
	// before the frame's load event — so poll for it instead of waiting for load.
	const FRAME_HOOK_INTERVAL_MS = 50
	let focusReclaim: { target: HTMLElement; until: number } | undefined
	let frameInteracted = false
	const hookedDocs = new WeakSet<Document>()
	let hookTimer: ReturnType<typeof setInterval> | undefined

	function textEntry(el: EventTarget | null): HTMLElement | undefined {
		if (el instanceof HTMLTextAreaElement || el instanceof HTMLInputElement) return el
		if (el instanceof HTMLElement && el.isContentEditable) return el
		return undefined
	}

	function armFocusReclaim() {
		const target = textEntry(document.activeElement)
		frameInteracted = false
		focusReclaim = target ? { target, until: Date.now() + FOCUS_RECLAIM_WINDOW_MS } : undefined
		if (!focusReclaim) return
		hookFrameDocument()
		clearInterval(hookTimer)
		hookTimer = setInterval(() => {
			hookFrameDocument()
			if (!focusReclaim || Date.now() > focusReclaim.until) {
				clearInterval(hookTimer)
				hookTimer = undefined
			}
		}, FRAME_HOOK_INTERVAL_MS)
	}

	// Whether the user has acted inside the frame is the whole question, and a click
	// or keypress in there reaches no listener out here — so read it off the frame's
	// own document (same-origin). These fire ahead of the focus move they cause,
	// which is what separates a user's click from focus the page took by itself.
	function hookFrameDocument() {
		try {
			const doc = frame?.contentDocument
			if (!doc || hookedDocs.has(doc)) return
			hookedDocs.add(doc)
			const mark = () => (frameInteracted = true)
			doc.addEventListener('pointerdown', mark, true)
			doc.addEventListener('keydown', mark, true)
		} catch {
			// Defensively cross-origin; the fallback below decides instead.
		}
	}

	function frameIsHooked(): boolean {
		try {
			const doc = frame?.contentDocument
			return !!doc && hookedDocs.has(doc)
		} catch {
			return false
		}
	}

	// Fallback for a document not hooked yet (one poll tick, or cross-origin): the
	// router parks focus on the frame's body, so anything else focused in there was
	// more likely clicked. Too coarse to rely on once the frame is hooked — a
	// previewed page that autofocuses a field of its own (Runs does) would read as a
	// user's click. An unreadable frame is not reclaimable, as in the catch.
	function frameFocusIsRouterReset(): boolean {
		try {
			const doc = frame?.contentDocument
			if (!doc) return false
			return !doc.activeElement || doc.activeElement === doc.body
		} catch {
			return false
		}
	}

	// Arms on mount and on every src change (a tab opened or re-pointed by the
	// assistant); reload() arms for the in-frame navigations it drives.
	$effect(() => {
		void iframeSrc
		if (!frame) return
		armFocusReclaim()
	})

	// Focus the user moves elsewhere in this document supersedes the armed target,
	// so a reclaim that lands late can never drag them back to the field they left.
	$effect(() => {
		function onFocusIn(e: FocusEvent) {
			if (!focusReclaim) return
			const target = textEntry(e.target)
			focusReclaim = target ? { ...focusReclaim, target } : undefined
		}
		document.addEventListener('focusin', onFocusIn, true)
		return () => document.removeEventListener('focusin', onFocusIn, true)
	})

	// The armed field losing focus is the only event this document gets out of the
	// transfer — nothing fires once focus is inside the frame, and Firefox does not
	// even blur the window for it.
	$effect(() => {
		function onFocusOut(e: FocusEvent) {
			const reclaim = focusReclaim
			if (!reclaim || e.target !== reclaim.target) return
			// Once the frame is hooked, `frameInteracted` below is the whole answer.
			// Until then, read the frame's focus now, while it still reflects the move
			// that just happened — a page autofocuses its own field a tick later.
			const looksAutomatic = frameIsHooked() || frameFocusIsRouterReset()
			// The verdict on this document waits a task, because the transfer into the
			// frame is still in flight here — Chrome still reports the body as
			// document.activeElement — and because focusing from inside the handler
			// moves activeElement but not the keyboard: the in-flight transfer lands
			// last, leaving the caret drawn here and the keystrokes going to the frame.
			setTimeout(() => {
				if (focusReclaim !== reclaim || !frame || document.activeElement !== frame) return
				if (frameInteracted || Date.now() > reclaim.until || !reclaim.target.isConnected) {
					focusReclaim = undefined
					return
				}
				if (!looksAutomatic) return
				reclaim.target.focus({ preventScroll: true })
			}, 0)
		}
		document.addEventListener('focusout', onFocusOut, true)
		return () => document.removeEventListener('focusout', onFocusOut, true)
	})
	$effect(() => () => clearInterval(hookTimer))

	const visibility = $derived(
		active ? 'z-10 opacity-100 pointer-events-auto' : 'z-0 opacity-0 pointer-events-none'
	)

	let flashing = $state(false)
	let flashTimer: ReturnType<typeof setTimeout> | undefined
	// Guard against the effect's non-pulse reruns (tab/runtime changes) firing a flash.
	let lastPulseNonce = -1
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
	<div class="absolute inset-0 flex flex-col min-h-0 bg-surface {visibility}" aria-hidden={!active}>
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
	<div class="absolute inset-0 flex flex-col min-h-0 bg-surface {visibility}" aria-hidden={!active}>
		{#if artifact}
			<ArtifactViewer {artifact} />
		{:else if !runtime?.manager.artifacts.loading}
			<div class="p-4 text-sm text-tertiary">This artifact is no longer available.</div>
		{/if}
		<!-- Overlay: the source editor's opaque bg would cover a ring on the container. -->
		<div
			class="pointer-events-none absolute inset-0 z-30 ring-2 ring-inset ring-border-accent transition-opacity duration-300 {flashing
				? 'opacity-100'
				: 'opacity-0'}"
			aria-hidden="true"
		></div>
	</div>
{:else if mounted}
	<iframe
		bind:this={frame}
		src={iframeSrc}
		onload={(e) => {
			const f = e.currentTarget as HTMLIFrameElement
			// Re-apply after load so a toggle that happened while the frame was
			// loading (its layout read the pre-toggle preference) isn't lost.
			applyPageIframeTheme(darkMode, f)
			hookFrameDocument()
			onLoad(f)
		}}
		title="Session preview: {label}"
		class="absolute inset-0 w-full h-full border-0 bg-surface {visibility}"
	></iframe>
{/if}
