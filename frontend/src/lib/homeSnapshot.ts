import { version } from '$app/environment'
import { readSessionHint, sessionStillConfirmed } from '$lib/lastSession'

// The markup of the last rendered Home, painted by `app.html` in place of its loading splash
// while the app boots, and replaced by the live page when it mounts. `app.html` reads it before
// any module loads, so the rules it applies live there, in the script that paints it; this side
// writes what that script checks. Same binding as the optimistic render (`lastSession.ts`): a
// snapshot is shown only under the session hint it was saved with, so the next person on a
// shared browser never sees the previous one's Home.

export const HOME_SNAPSHOT_KEY = 'wm-home-snapshot'
/** Bumped when what the snapshot holds changes shape; `app.html` checks the same number. */
export const HOME_SNAPSHOT_FORMAT = 1
const MAX_HTML_CHARS = 512 * 1024
const MAX_INLINE_STYLE_CHARS = 64 * 1024

export interface HomeSnapshot {
	format: number
	/** The build it was rendered by: its stylesheets are content-hashed and go with it. */
	build: string
	/** The session-hint cookie's name, which the dev proxy renames per backend. */
	cookie: string
	hint: string
	workspace: string
	width: number
	dark: boolean
	htmlClass: string
	background: string
	stylesheets: string[]
	styles: string[]
	html: string
	savedAt: number
}

/** Everything a live Home holds that the snapshot must not: values typed, and anything open. */
const DROPPED = [
	'script',
	'noscript',
	'template',
	'iframe',
	'frame',
	'object',
	'embed',
	'canvas',
	'video',
	'audio',
	'link',
	'dialog',
	'[popover]',
	'[role="dialog"]',
	'[role="alertdialog"]',
	'[role="menu"]',
	'[role="listbox"]',
	'[role="tooltip"]',
	'._toastContainer',
	'#svelte-announcer',
	'#svelte-global-loader',
	'#wm-home-snapshot'
].join(',')

const SVG_NS = 'http://www.w3.org/2000/svg'

/**
 * The app's markup without what could carry a value or act, as XML: the live tree nests what the
 * HTML parser would restructure (an anchor inside an anchor), and XML parses back as it was.
 */
function snapshotMarkup(root: Element): string {
	const clone = document.createElement('div')
	clone.append(...Array.from(root.childNodes, (n) => n.cloneNode(true)))
	// A part the live page draws differently while it boots carries what it draws then, and the
	// snapshot shows that instead, so the live page's first frame matches it.
	clone.querySelectorAll('[data-wm-snapshot-slot]').forEach((slot) => {
		const standIn = slot.querySelector(':scope > [data-wm-snapshot-stand-in]')
		if (standIn) slot.replaceChildren(...Array.from(standIn.childNodes))
	})
	clone
		.querySelectorAll<HTMLElement>('[data-wm-snapshot-hidden]')
		.forEach((el) => (el.style.visibility = 'hidden'))
	clone.querySelectorAll(DROPPED).forEach((el) => el.remove())
	clone.querySelectorAll('*').forEach((el) => {
		for (const { name, value } of Array.from(el.attributes)) {
			if (
				name.startsWith('on') ||
				// The live page owns these ids while both are in the document.
				(name === 'id' && el.namespaceURI !== SVG_NS) ||
				(/^(href|src|xlink:href|action|formaction)$/.test(name) && /^\s*javascript:/i.test(value))
			)
				el.removeAttribute(name)
		}
		if (el instanceof HTMLInputElement) el.removeAttribute('value')
		else if (el instanceof HTMLTextAreaElement) el.textContent = ''
		else if (el.hasAttribute('contenteditable')) el.replaceChildren()
	})
	return new XMLSerializer().serializeToString(clone)
}

function linkedStylesheets(): string[] {
	return Array.from(document.querySelectorAll<HTMLLinkElement>('link[rel="stylesheet"]'))
		.map((l) => l.getAttribute('href') ?? '')
		.filter(Boolean)
}

// SvelteKit keeps every visited route's stylesheets linked, so the stylesheets a Home reached in
// the app sees are not the ones a reload of it needs.
let navigatedInApp = false
/** Called by the root layout before every client-side navigation. */
export function noteClientNavigation(): void {
	navigatedInApp = true
}

/** Only the plain Home URL: a search or another tab renders something else. */
function isPlainHomeUrl(url: URL): boolean {
	let other = false
	url.searchParams.forEach((_, k) => (other ||= k !== 'workspace'))
	return url.pathname === '/' && !other
}

/**
 * Saves what Home shows now for `workspace`, if the session it was rendered for is still the
 * browser's. Anything that cannot be painted faithfully on the next load clears the previous
 * snapshot instead of keeping one that no longer matches.
 */
export function saveHomeSnapshot(
	workspace: string,
	userWorkspace: string | undefined,
	stylesheets: string[] = linkedStylesheets()
): void {
	try {
		const hint = readSessionHint()
		if (
			!hint ||
			!sessionStillConfirmed() ||
			userWorkspace !== workspace ||
			window.parent !== window ||
			!isPlainHomeUrl(new URL(location.href)) ||
			homeSnapshotShown()
		)
			return
		const app = document.getElementById('wm-app')
		if (!app) return
		const styles = Array.from(document.head.querySelectorAll('style'))
			.filter((s) => !s.hasAttribute('data-wm-shell') && !s.hasAttribute('data-wm-home-snapshot'))
			.map((s) => s.textContent ?? '')
		const html = snapshotMarkup(app)
		// Unbundled dev styles arrive as megabytes of inline <style>: no snapshot there.
		if (
			html.length > MAX_HTML_CHARS ||
			styles.reduce((n, s) => n + s.length, 0) > MAX_INLINE_STYLE_CHARS
		) {
			clearHomeSnapshot()
			return
		}
		const snapshot: HomeSnapshot = {
			format: HOME_SNAPSHOT_FORMAT,
			build: version,
			cookie: __WM_SESSION_HINT__,
			hint,
			workspace,
			width: window.innerWidth,
			dark: document.documentElement.classList.contains('dark'),
			htmlClass: document.documentElement.className,
			// The cover is opaque so the app mounting under it never shows through.
			background: getComputedStyle(document.documentElement).backgroundColor,
			stylesheets,
			styles,
			html,
			savedAt: Date.now()
		}
		localStorage.setItem(HOME_SNAPSHOT_KEY, JSON.stringify(snapshot))
	} catch {
		clearHomeSnapshot()
	}
}

/**
 * Saves Home while it is mounted: once its fresh data has had time to land, and again as the
 * page is left or hidden. Only while nothing has been pressed on it since it mounted: a reload
 * renders Home with folders collapsed and the composer closed, and a snapshot of another state
 * would jump at the hand-over. Only for a Home the document loaded on, with the stylesheets linked
 * as it mounted: what a reload's first frame has. Returns the cleanup.
 */
export function watchHomeForSnapshot(
	current: () => { workspace: string | undefined; userWorkspace: string | undefined }
): () => void {
	if (navigatedInApp) return () => {}
	const stylesheets = linkedStylesheets()
	let touched = false
	const touch = () => (touched = true)
	// A shortcut or a modifier changes nothing on Home, and a keyboard reload or an alt-tab would
	// otherwise cancel the save it leads to.
	const touchKey = (e: KeyboardEvent) => {
		if (!e.ctrlKey && !e.metaKey && !e.altKey && !/^(F\d+|Control|Meta|Alt|Shift|Tab)$/.test(e.key))
			touched = true
	}
	const save = () => {
		const { workspace, userWorkspace } = current()
		if (!touched && workspace) saveHomeSnapshot(workspace, userWorkspace, stylesheets)
	}
	const onHidden = () => {
		if (document.visibilityState === 'hidden') save()
	}
	const settled = setTimeout(save, 2000)
	document.addEventListener('pointerdown', touch, true)
	document.addEventListener('keydown', touchKey, true)
	document.addEventListener('visibilitychange', onHidden)
	window.addEventListener('pagehide', save)
	return () => {
		clearTimeout(settled)
		document.removeEventListener('pointerdown', touch, true)
		document.removeEventListener('keydown', touchKey, true)
		document.removeEventListener('visibilitychange', onHidden)
		window.removeEventListener('pagehide', save)
	}
}

export function clearHomeSnapshot(): void {
	try {
		localStorage.removeItem(HOME_SNAPSHOT_KEY)
	} catch {}
}

/** Whether `app.html` painted a snapshot that the live page has not replaced yet. */
export function homeSnapshotShown(): boolean {
	return typeof document !== 'undefined' && !!document.getElementById('wm-home-snapshot')
}

/**
 * Replaces the snapshot with the live page. Called as the live page mounts, before the browser
 * can paint it, so the two never share a frame.
 */
export function releaseHomeSnapshot(): void {
	document.getElementById('wm-home-snapshot')?.remove()
	// Its stylesheet links stay: they are the app's own now, since the build's preload helper
	// does not add a stylesheet the document already links.
	document.querySelectorAll('style[data-wm-home-snapshot]').forEach((el) => el.remove())
	document.documentElement.removeAttribute('data-wm-home-snapshot')
}
