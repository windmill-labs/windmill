import { get } from 'svelte/store'
import { goto } from '$lib/navigation'
import { workspaceStore } from '$lib/stores'
import {
	blankHomeDraft,
	createSession,
	homeDraft,
	opensOnHome,
	selectSession,
	sessionInCurrentFamily,
	sessionLastActivityAt,
	sessionPageHref,
	sessionState,
	setSessionAutoSend,
	setSessionDraftPrompt,
	setSessionPendingWorkspace,
	showOnHome,
	syncWorkspaceTo,
	type Session,
	type SessionTarget
} from './sessionState.svelte'
import { sessionTargetHref, withPreviewParams } from './sessionMode.svelte'
import { homeSidebarList, type SidebarList } from './sidebarList.svelte'
import { hrefAfterWorkspaceSwitch } from '../sidebar/workspaceSwitchUrl'
import { pinHomeComposer } from '../home/homeComposerPin.svelte'
import { homeComposerContext } from './homeComposerContext.svelte'
// Type-only: erased at compile time, so the component graph stays out of this
// navigation seam (see the dynamic import in openEditorInSession).
import type { OpenInSessionSource } from './OpenInSessionButton.svelte'

// The Workspace ⇄ AI Sessions switch moves between two places worked in side by side,
// and comes back to each where it was left. Plain module state: read only in click
// handlers, never rendered. A reload forgets both, and the switch then falls back to
// Home and to the family's most recently used session.

// The last page shown with the workspace nav beside it, Home included, as a pathname
// with its search, and the workspace it was shown in.
let workspacePage: { href: string; workspace: string | undefined } | undefined
// The session last opened, on its page or in Home's composer.
let lastSessionId: string | undefined

/** The page Workspace returns to. Called on every arrival with the workspace nav shown,
 * and by an in-place URL rewrite the router doesn't report as a navigation. */
export function rememberWorkspacePage(href: string): void {
	workspacePage = { href, workspace: get(workspaceStore) ?? undefined }
}

/** Where a navigation landed, for the switch to come back to. Landing anywhere but Home
 * also sets the list Home will show. */
export function noteArrival(url: URL): void {
	if (url.pathname.startsWith('/sessions')) {
		homeSidebarList.val = 'sessions'
		lastSessionId = url.searchParams.get('session') ?? lastSessionId
		return
	}
	if (url.pathname !== '/') homeSidebarList.val = 'workspace'
	if (homeSidebarList.val === 'workspace') rememberWorkspacePage(url.pathname + url.search)
}

/** The switch on Home, which belongs to both lists: it swaps the list and stays. */
export function swapHomeList(list: SidebarList, homeHref: string): void {
	homeSidebarList.val = list
	if (list === 'workspace') rememberWorkspacePage(homeHref)
	// The AI side of Home is its composer: going there puts the caret in it. After the click
	// has played out, or the switch button it landed on takes focus back.
	else setTimeout(() => homeComposerContext.focus(), 0)
}

// The session last opened, else the family's most recently used one. Scoped to the
// family: reviving another family's session would pull its scope into this one.
function resumableSession(): Session | undefined {
	const usable = (s: Session) => !s.archived && sessionInCurrentFamily(s)
	const last = sessionState.sessions.find((s) => s.id === lastSessionId)
	if (last && usable(last)) return last
	return sessionState.sessions
		.filter((s) => usable(s) && !s.transient)
		.sort((a, b) => sessionLastActivityAt(b) - sessionLastActivityAt(a))[0]
}

/** AI Sessions, from anywhere but Home: the session to resume on its page, or in Home's
 * composer for an unsent draft, else a blank draft there. `replace` for the sessions
 * page's own redirects, where Back must not return to the URL that bounced here. */
export async function resumeAiSessions(opts?: { replace?: boolean }): Promise<void> {
	const session = resumableSession()
	if (!session || opensOnHome(session)) {
		await toHome((session ?? blankHomeDraft()).id, opts)
		return
	}
	selectSession(session.id)
	await goto(sessionPageHref(session.id), { replaceState: opts?.replace ?? false })
}

/** Workspace, from a session's page: the page it was left on, else Home. One left in
 * another workspace than the header's is moved by the workspace picker's rule: restored
 * as it was, its item could be missing here or its params switch the workspace back. */
export async function returnToWorkspace(): Promise<void> {
	homeSidebarList.val = 'workspace'
	const left = workspacePage
	const ws = get(workspaceStore) ?? undefined
	let href = left?.href ?? '/'
	if (left && ws && left.workspace !== ws) href = hrefAfterWorkspaceSwitch(left.href, ws)
	await goto(href)
}

// A plain session starts on Home, and an unsent draft is shown there rather than on a
// page of its own, with the session list beside it. Asking for one puts the composer
// back on Home if the reader had unpinned it: there is nowhere else to type it.
async function toHome(id: string, opts?: { replace?: boolean }): Promise<void> {
	homeSidebarList.val = 'sessions'
	lastSessionId = id
	showOnHome(id, { focus: true })
	pinHomeComposer()
	// `goto` has no same-URL short-circuit, so navigating from Home would push a
	// duplicate history entry; showOnHome already remounts the composer there.
	if (typeof window !== 'undefined' && window.location.pathname === '/') return
	await goto('/', { replaceState: opts?.replace ?? false })
}

/** New session: Home's composer, blank and focused. It acts on the header's workspace,
 * switched to `workspaceId` first when given. A draft already typed stays in the list. */
export async function openNewSession(workspaceId?: string): Promise<void> {
	// Home's current draft stays where it is: the switch below must not take it along.
	homeDraft.id = undefined
	if (workspaceId) syncWorkspaceTo(workspaceId)
	await toHome(blankHomeDraft().id)
}

/** A session picked from a list: an unsent draft in Home's composer, any other on its page. */
export async function openSession(session: Session): Promise<void> {
	if (opensOnHome(session)) return openDraftOnHome(session.id)
	selectSession(session.id)
	await goto(sessionPageHref(session.id))
}

/** An unsent draft, opened in Home's composer. `replace` for a redirect off the sessions
 * page, where Back must not return to the URL that bounced here. */
export async function openDraftOnHome(id: string, opts?: { replace?: boolean }): Promise<void> {
	await toHome(id, opts)
}

/** Where to go once the session on screen is gone (archived, deleted, not in this browser):
 * Home, showing whichever draft it would pick anyway. */
export async function leaveForHome(opts?: { replace?: boolean }): Promise<void> {
	homeDraft.id = undefined
	homeDraft.requests++
	if (typeof window !== 'undefined' && window.location.pathname === '/') return
	await goto('/', { replaceState: opts?.replace ?? false })
}

// Open a fresh AI session showing an editor (flow/script/raw_app) in its preview,
// then route to its page. The preview loads the item from its live draft,
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
	await goto(await sessionHrefFor(url, workspaceId, opts))
}

/** A fresh session's page, with an editor in its preview. For a `load` that sends the
 * user there by redirect, where a `goto` would start a second navigation. */
export function editorSessionHref(
	target: SessionTarget,
	workspaceId?: string,
	previewParams?: Record<string, string>
): Promise<string> {
	return sessionHrefFor(withPreviewParams(sessionTargetHref(target), previewParams), workspaceId)
}

async function sessionHrefFor(
	url: string | undefined,
	workspaceId?: string,
	opts?: { seedPrompt?: string; autoSend?: boolean }
): Promise<string> {
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
	}
	selectSession(session.id)
	return sessionPageHref(session.id)
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
// global search's "Ask AI"). Always a new session, so the seed cannot overwrite
// a prompt the user has already typed into another one.
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
	await goto(sessionPageHref(session.id))
}
