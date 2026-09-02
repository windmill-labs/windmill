import { get } from 'svelte/store'
import { goto } from '$lib/navigation'
import { workspaceStore } from '$lib/stores'
import {
	createSession,
	selectSession,
	sessionInCurrentFamily,
	sessionLastActivityAt,
	sessionState,
	setSessionAutoSend,
	setSessionDraftPrompt,
	setSessionPendingWorkspace,
	type Session,
	type SessionTarget
} from './sessionState.svelte'
import { sessionTargetHref, withPreviewParams } from './sessionMode.svelte'
import { parsePreviewItemRoute, type PreviewItemRoute } from './previewPaths'
import { findMountedOpenInSessionSource } from './openInSessionContext'
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
// Whether `lastNavRoute` has been offered as a new session's starting point since
// it was remembered (see takeNewSessionSeed).
let navRouteOffered = false

export function rememberNavRoute(pathnameWithSearch: string): void {
	lastNavRoute = pathnameWithSearch
	navRouteOffered = false
}

/** An item editor the user reached session mode from, as the in-app href a new
 * session's preview can open, plus the item it edits (for naming it). */
export type NewSessionSeed = { url: string; route: PreviewItemRoute }

// The item (flow, script, app) the user came to session mode from, or undefined
// when they came from anywhere else. A "New session" asked for after arriving
// from an item is usually a session about that item, but the arrival route
// (`enterSessionMode`) resumes whatever session was open, so the picker offers
// the item back. Offered once per remembered route: the first "New session" of
// a visit gets the question, however long the visit has run and whatever was
// done in between; later ones start empty without asking, and a dismissed offer
// is not repeated until the user leaves for another page and comes back.
export function takeNewSessionSeed(): NewSessionSeed | undefined {
	if (navRouteOffered) return undefined
	navRouteOffered = true
	const route = parsePreviewItemRoute(lastNavRoute)
	return route ? { url: lastNavRoute, route } : undefined
}

// The session entering session mode resumes: the active one if selected, else
// the most recent non-archived one. Scoped to the active workspace family:
// reviving a session from another family would pull that family's scope
// (sidebar list, "Acting on" workspace) into the one the user is actually in.
function resumableSession(): Session | undefined {
	const current = sessionState.currentSessionId
		? sessionState.sessions.find((s) => s.id === sessionState.currentSessionId)
		: undefined
	return (
		(current && sessionInCurrentFamily(current) ? current : undefined) ??
		sessionState.sessions.find((s) => !s.archived && sessionInCurrentFamily(s))
	)
}

// Enter session mode: resume the session `resumableSession` picks, else spin up
// a fresh one — then route to it.
// `replace` swaps the current history entry instead of pushing — for the
// sessions page's family reconcile, where Back must not return to the
// redirected-away URL just to bounce here again.
export async function enterSessionMode(opts?: { replace?: boolean }): Promise<void> {
	const target = resumableSession() ?? createSession()
	selectSession(target.id)
	await goto(`/sessions?session_name=${encodeURIComponent(target.name)}`, {
		replaceState: opts?.replace ?? false
	})
}

// How long the resumable session may have sat idle before entering from an item
// editor starts a session on that item instead. Long enough that stepping out to
// the editor in the middle of a conversation comes back to the same chat; short
// enough that a session left since the previous day is not taken for the task
// the user is now on.
const RESUME_IDLE_LIMIT_MS = 60 * 60 * 1000

// Enter session mode from the navigation rail. Coming from an item editor with
// no session to resume, or one idle past RESUME_IDLE_LIMIT_MS, opens a fresh
// session on that item straight away: a session that old is rarely what a visit
// from a flow or app is about, and landing in it would only lead to "New
// session" and the offer takeNewSessionSeed makes. Anything else resumes as
// enterSessionMode does. Rejects when the editor could not persist its draft,
// so the caller can stay on the page and say so.
export async function enterSessionModeFromNav(): Promise<void> {
	const route = parsePreviewItemRoute(lastNavRoute)
	if (route) {
		const resumable = resumableSession()
		if (!resumable || Date.now() - sessionLastActivityAt(resumable) > RESUME_IDLE_LIMIT_MS) {
			// The editor's own hand-off is what its "Open in AI session" button uses:
			// it persists the draft the preview loads (an edit still inside the
			// autosave debounce, the row of a never-saved new item) and names the
			// workspace the item lives in. Only an item with no such editor on
			// screen (a legacy app, a detail page) is opened by route, as last
			// persisted, in the workspace the route was scoped to.
			const source = findMountedOpenInSessionSource(route)
			if (source) await openSourceInSession(source)
			else await openPageInSession(lastNavRoute, workspaceParamOf(lastNavRoute))
			return
		}
	}
	await enterSessionMode()
}

function workspaceParamOf(pathnameWithSearch: string): string | undefined {
	const query = pathnameWithSearch.split('?')[1]
	return query ? (new URLSearchParams(query).get('workspace') ?? undefined) : undefined
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
	opts?: { seedPrompt?: string; autoSend?: boolean }
): Promise<void> {
	await openInSession(
		withPreviewParams(sessionTargetHref(target), previewParams),
		workspaceId,
		opts
	)
}

// Open a fresh AI session showing an in-app href in its preview: a workspace
// page (Runs, a trigger list), which is not an editable item and so has no
// SessionTarget, or a location captured as the user left it.
export async function openPageInSession(
	href: string,
	workspaceId?: string,
	opts?: { seedPrompt?: string; autoSend?: boolean }
): Promise<void> {
	await openInSession(href, workspaceId, opts)
}

async function openInSession(
	url: string | undefined,
	workspaceId?: string,
	opts?: { seedPrompt?: string; autoSend?: boolean }
): Promise<void> {
	// Seed the fresh session's preview with a single tab on `url` so it opens
	// straight onto what the caller wants (resetSessionPreviewTabs also writes
	// through a live runtime if one already exists for this id).
	const session = createSession()
	if (workspaceId) setSessionPendingWorkspace(session.id, workspaceId)
	if (opts?.seedPrompt) {
		setSessionDraftPrompt(session.id, opts.seedPrompt)
		if (opts.autoSend) setSessionAutoSend(session.id)
	}
	if (url) {
		// Dynamic import: a static one would drag the runtime's heavy graph
		// (chat manager → monaco) into this thin navigation seam, breaking its
		// node-run unit tests.
		const { resetSessionPreviewTabs } = await import('./sessionRuntime.svelte')
		resetSessionPreviewTabs(session.id, url)
		// Hand-offs seed the page they leave, so the arrival has opened the item
		// itself and "New session" need not offer it again.
		navRouteOffered = true
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
	overrides?: { previewParams?: Record<string, string>; seedPrompt?: string; autoSend?: boolean }
): Promise<void> {
	await source.beforeOpen?.()
	const opts = {
		seedPrompt: overrides?.seedPrompt ?? source.seedPrompt,
		autoSend: overrides?.autoSend ?? source.autoSend
	}
	if (source.target) {
		await openEditorInSession(
			source.target,
			source.workspaceId,
			overrides?.previewParams ?? source.previewParams,
			opts
		)
		return
	}
	const href = source.page?.()
	if (href) await openPageInSession(href, source.workspaceId, opts)
}

// Open a fresh session on no particular item, with `prompt` pre-filled in the
// composer. For entry points that carry a question rather than a target (the
// global search's "Ask AI"). Always a new session rather than the most recent
// one (`enterSessionMode`), so the seed cannot overwrite a prompt the user has
// already typed into a session they are mid-way through.
export async function startSessionWithPrompt(
	prompt: string,
	opts?: { autoSend?: boolean }
): Promise<void> {
	// No setSessionPendingWorkspace: createSession already picked the workspace,
	// steering off a root the user cannot deploy to onto its dev. Overwriting it
	// with the raw current workspace would land the session where it cannot edit.
	const session = createSession()
	setSessionDraftPrompt(session.id, prompt)
	// An empty prompt has nothing to send; leave the composer focused instead.
	if (opts?.autoSend && prompt.trim()) setSessionAutoSend(session.id)
	selectSession(session.id)
	await goto(`/sessions?session_name=${encodeURIComponent(session.name)}`)
}
