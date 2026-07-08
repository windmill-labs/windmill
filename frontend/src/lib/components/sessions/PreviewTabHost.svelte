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
	import { resolvePreviewTab } from './previewRouter'
	import { withMenuHidden } from './sessionMode.svelte'
	import ScriptEditorView from './ScriptEditorView.svelte'
	import FlowEditorView from './FlowEditorView.svelte'
	import RawAppEditorView from './RawAppEditorView.svelte'

	let {
		tab,
		session,
		runtime,
		active,
		mounted,
		label,
		onNavigate,
		onLoad
	}: {
		tab: SessionPreviewTab
		session: Session | undefined
		runtime: SessionRuntime | undefined
		/** Visible tab — only one is at a time; the rest stay mounted but hidden. */
		active: boolean
		/** Lazy-mount gate: content only renders once the tab has been activated. */
		mounted: boolean
		/** Short tab label, for the iframe title. */
		label: string
		/** A link click inside a live editor re-points the session target + tab. */
		onNavigate: (item: WorkspaceItem) => void
		/** Iframe finished loading — the page reads back its observed location. */
		onLoad: (frame: HTMLIFrameElement) => void
	} = $props()

	// Editor vs iframe is decided purely from the tab URL (see resolvePreviewTab):
	// any editable item (script/flow/raw app) mounts its own live editor.
	const slot = $derived(resolvePreviewTab(tab.url))
	const workspaceId = $derived(
		session ? (getEffectiveWorkspaceId(session) ?? $workspaceStore ?? '') : ''
	)
	const isActiveSession = $derived(!!session && sessionState.currentSessionId === session.id)

	let frame: HTMLIFrameElement | undefined = $state()

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
				initialTestPanelCollapsed
			/>
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
		onload={(e) => onLoad(e.currentTarget as HTMLIFrameElement)}
		title="Session preview: {label}"
		class="absolute inset-0 w-full h-full border-0 bg-surface {visibility}"
	></iframe>
{/if}
