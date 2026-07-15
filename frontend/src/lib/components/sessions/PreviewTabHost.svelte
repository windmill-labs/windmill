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
	import { resolvePreviewTab, parsePreviewItemRoute } from './previewRouter'
	import { withMenuHidden } from './sessionMode.svelte'
	import ScriptEditorView from './ScriptEditorView.svelte'
	import FlowEditorView from './FlowEditorView.svelte'
	import RawAppEditorView from './RawAppEditorView.svelte'
	import PipelineEditorView from './PipelineEditorView.svelte'

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
			// scoping can only live in the URL). replace() forces the load even when
			// the target equals the current URL.
			win.location.replace(withMenuHidden(tab.loc || tab.url, workspaceId || undefined))
		} catch {
			// Cross-navigation timing — skip; the next mutation reloads again.
		}
	}

	const visibility = $derived(
		active ? 'z-10 opacity-100 pointer-events-auto' : 'z-0 opacity-0 pointer-events-none'
	)
</script>

{#if slot.kind === 'editor' && mounted && runtime}
	<div class="absolute inset-0 flex flex-col min-h-0 bg-surface {visibility}" aria-hidden={!active}>
		{#if slot.editorKind === 'flow'}
			<FlowEditorView
				{runtime}
				path={slot.path}
				{workspaceId}
				{onNavigate}
				{isActiveSession}
				{active}
			/>
		{:else if slot.editorKind === 'script'}
			<ScriptEditorView
				{runtime}
				path={slot.path}
				{workspaceId}
				{onNavigate}
				{isActiveSession}
				{active}
				{fullscreen}
			/>
		{:else if slot.editorKind === 'pipeline'}
			<PipelineEditorView {runtime} path={slot.path} {workspaceId} {isActiveSession} {active} />
		{:else}
			<RawAppEditorView
				{runtime}
				path={slot.path}
				{workspaceId}
				{onNavigate}
				{isActiveSession}
				{active}
			/>
		{/if}
	</div>
{:else if mounted}
	<iframe
		bind:this={frame}
		src={withMenuHidden(tab.url, workspaceId || undefined)}
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
