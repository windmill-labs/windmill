import { goto } from '$lib/navigation'
import {
	createSession,
	selectSession,
	sessionInCurrentFamily,
	sessionState,
	setSessionPendingWorkspace,
	setSessionTarget,
	type SessionTarget
} from './sessionState.svelte'
import { sessionTargetHref } from './sessionMode.svelte'

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
	await goto(lastNavRoute || '/')
}

// Open a fresh AI session pre-targeted at an editor (flow/script/raw_app), then
// route into session mode. The session preview loads that editor via its target,
// so the caller MUST persist any unsaved edits first (e.g. save a draft) for the
// preview to reflect the live state. `workspaceId` scopes the session to the
// editor's workspace (instead of createSession's root default) so it opens the
// same flow/script the user was editing.
export async function openEditorInSession(
	target: SessionTarget,
	workspaceId?: string
): Promise<void> {
	// createSession() reuses an existing transient draft, which may still be
	// pointed at a *different* item — and its preview tabs (persisted with the
	// draft and/or held by a live runtime) keep showing that old target unless
	// the tab model is reset along with the target field.
	const session = createSession()
	const retargeted = session.target?.kind !== target.kind || session.target?.path !== target.path
	if (workspaceId) setSessionPendingWorkspace(session.id, workspaceId)
	setSessionTarget(session.id, target)
	const url = sessionTargetHref(target)
	if (url && retargeted) {
		// Dynamic import: a static one would drag the runtime's heavy graph
		// (chat manager → monaco) into this thin navigation seam, breaking its
		// node-run unit tests.
		const { resetSessionPreviewTabs } = await import('./sessionRuntime.svelte')
		resetSessionPreviewTabs(session.id, url)
	}
	selectSession(session.id)
	await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
}
