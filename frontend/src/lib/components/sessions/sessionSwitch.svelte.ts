import { get } from 'svelte/store'
import { goto } from '$lib/navigation'
import { workspaceStore } from '$lib/stores'
import {
	createSession,
	selectSession,
	sessionInCurrentFamily,
	sessionState,
	setSessionDraftPrompt,
	setSessionPendingWorkspace,
	type SessionTarget
} from './sessionState.svelte'
import { sessionTargetHref, withPreviewParams } from './sessionMode.svelte'
// Type-only: erased at compile time, so the component graph stays out of this
// navigation seam (see the dynamic import in openEditorInSession).
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

// The session/navigation switch turns the global rail into either the workspace
// navigation (navigation mode) or the sessions sidebar (session mode). Session
// mode is route-derived — it is exactly "on the /sessions page" — so the switch
// is just navigation in and out of that route.

// The last non-session route the user was on, so exiting session mode returns
// them where they were instead of dumping them on the home page. Stored as a
// base-prefixed pathname (+ search); `goto` is base-idempotent so re-prefixing
// is safe. Plain module state: it is read only inside click handlers, never
// rendered, so it needs no reactivity.
let lastNavRoute = '/'

export function rememberNavRoute(pathnameWithSearch: string): void {
	lastNavRoute = pathnameWithSearch
}

// Enter session mode: open the active session if one is selected, else the most
// recent non-archived session, else spin up a fresh one — then route to it.
// Restore candidates are scoped to the active workspace family: reviving a
// session from another family would pull that family's scope (sidebar list,
// "Acting on" workspace) into the one the user is actually in.
// `replace` swaps the current history entry instead of pushing — for the
// sessions page's family reconcile, where Back must not return to the
// redirected-away URL just to bounce here again.
export async function enterSessionMode(opts?: { replace?: boolean }): Promise<void> {
	const current = sessionState.currentSessionId
		? sessionState.sessions.find((s) => s.id === sessionState.currentSessionId)
		: undefined
	const target =
		(current && sessionInCurrentFamily(current) ? current : undefined) ??
		sessionState.sessions.find((s) => !s.archived && sessionInCurrentFamily(s)) ??
		createSession()
	selectSession(target.id)
	await goto(`/sessions?session_name=${encodeURIComponent(target.name)}`, {
		replaceState: opts?.replace ?? false
	})
}

// Exit session mode: back to the last navigation route (home as a fallback).
export async function exitSessionMode(): Promise<void> {
	let target = lastNavRoute || '/'
	// The remembered route can carry a stale `?workspace=` — the layout's
	// onQueryChange re-applies that param, so restoring the route would
	// silently check the app back out to a workspace the user has since
	// left. The rest of the route is scoped to that stale workspace too,
	// so fall back to home rather than rewriting the param.
	const wsParam = /[?&]workspace=([^&]*)/.exec(target)?.[1]
	const ws = get(workspaceStore)
	if (ws && wsParam && decodeURIComponent(wsParam) !== ws) {
		target = '/'
	}
	await goto(target)
}

// Open a fresh AI session showing an editor (flow/script/raw_app) in its preview,
// then route into session mode. The preview loads the item from its live draft,
// so the caller MUST persist any unsaved edits first (e.g. save a draft) for the
// preview to reflect the live state. `workspaceId` scopes the session to the
// editor's workspace (instead of createSession's root default) so it opens the
// same flow/script the user was editing. `previewParams` ride on the tab URL to
// tell the previewed editor where to open (a flow's `selected` step).
export async function openEditorInSession(
	target: SessionTarget,
	workspaceId?: string,
	previewParams?: Record<string, string>,
	opts?: { seedPrompt?: string }
): Promise<void> {
	// Seed the fresh session's preview with a single tab on `target` so it opens
	// straight onto the editor the caller wants (resetSessionPreviewTabs also
	// writes through a live runtime if one already exists for this id).
	const session = createSession()
	if (workspaceId) setSessionPendingWorkspace(session.id, workspaceId)
	if (opts?.seedPrompt) setSessionDraftPrompt(session.id, opts.seedPrompt)
	const url = withPreviewParams(sessionTargetHref(target), previewParams)
	if (url) {
		// Dynamic import: a static one would drag the runtime's heavy graph
		// (chat manager → monaco) into this thin navigation seam, breaking its
		// node-run unit tests.
		const { resetSessionPreviewTabs } = await import('./sessionRuntime.svelte')
		resetSessionPreviewTabs(session.id, url)
	}
	selectSession(session.id)
	await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
}

// Open an editor's own hand-off, running its `beforeOpen` (which persists the
// draft the preview loads) first. Callers that drive the hand-off imperatively
// go through this rather than `openEditorInSession` so they cannot skip that
// step; `OpenInSessionButton` is the declarative equivalent.
export async function openSourceInSession(
	source: OpenInSessionSource,
	overrides?: { previewParams?: Record<string, string>; seedPrompt?: string }
): Promise<void> {
	await source.beforeOpen?.()
	await openEditorInSession(
		source.target,
		source.workspaceId,
		overrides?.previewParams ?? source.previewParams,
		{ seedPrompt: overrides?.seedPrompt ?? source.seedPrompt }
	)
}

// Open a fresh session on no particular item, with `prompt` pre-filled in the
// composer. For entry points that carry a question rather than a target (the
// global search's "Ask AI"). Always a new session rather than the most recent
// one (`enterSessionMode`), so the seed cannot overwrite a prompt the user has
// already typed into a session they are mid-way through.
export async function startSessionWithPrompt(prompt: string): Promise<void> {
	// No setSessionPendingWorkspace: createSession already picked the workspace,
	// steering off a root the user cannot deploy to onto its dev. Overwriting it
	// with the raw current workspace would land the session where it cannot edit.
	const session = createSession()
	setSessionDraftPrompt(session.id, prompt)
	selectSession(session.id)
	await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
}
