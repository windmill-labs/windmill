import { describe, it, expect, beforeEach, vi } from 'vitest'
import { version } from '$app/environment'
import type { UserExt } from '$lib/stores'
import { forgetLastSession, rememberUser } from './lastSession'
import {
	HOME_SNAPSHOT_KEY,
	releaseHomeSnapshot,
	saveHomeSnapshot,
	type HomeSnapshot
} from './homeSnapshot'
import appHtml from '../app.html?raw'

// The rules are `app.html`'s: run its own scripts against what `saveHomeSnapshot` writes.
function inlineScript(id: string): string {
	const m = new RegExp(`<script id="${id}">([\\s\\S]*?)</script>`).exec(appHtml)
	if (!m) throw new Error(`no script #${id} in app.html`)
	return m[1].replace('%sveltekit.version%', version)
}
// Node's own experimental `localStorage` global shadows jsdom's, and is unset without a file.
function memoryStorage(): Storage {
	const m = new Map<string, string>()
	return {
		getItem: (k) => m.get(k) ?? null,
		setItem: (k, v) => void m.set(k, String(v)),
		removeItem: (k) => void m.delete(k),
		clear: () => m.clear(),
		key: (i) => [...m.keys()][i] ?? null,
		get length() {
			return m.size
		}
	}
}
for (const name of ['localStorage', 'sessionStorage'] as const)
	Object.defineProperty(globalThis, name, { value: memoryStorage(), configurable: true })

const headScript = inlineScript('wm-home-snapshot-head')
const bodyScript = inlineScript('wm-home-snapshot-body')

const user = { email: 'a@x.dev', username: 'a', workspace_id: 'demo' } as unknown as UserExt
const setHint = (v: string | undefined) => {
	for (const c of document.cookie.split(/;\s*/)) {
		const name = c.split('=')[0]
		if (name) document.cookie = `${name}=; expires=Thu, 01 Jan 1970 00:00:00 GMT`
	}
	if (v) document.cookie = `${__WM_SESSION_HINT__}=${v}`
}

function renderHome() {
	document.documentElement.removeAttribute('data-wm-home-snapshot')
	document.head.innerHTML = '<link rel="stylesheet" href="/_app/immutable/assets/app.css">'
	document.body.innerHTML = `<div id="wm-app"><div id="svelte-global-loader"></div><div id="content">
		<a href="/scripts/get/f/a">f/a</a><a href="javascript:alert(1)" onclick="alert(1)">x</a>
		<input value="typed"><textarea>draft</textarea><div contenteditable>draft</div>
		<div role="menu">menu</div><svg><g id="g1"></g></svg></div></div>`
}

/** Runs app.html's scripts as a load of `url` would; what it painted, if anything. */
function boot(url: string): { painted: HTMLElement | null; written: string } {
	history.replaceState(null, '', url)
	;(window as any).__wmHomeSnapshot = undefined
	document.documentElement.removeAttribute('data-wm-home-snapshot')
	document.head.innerHTML = ''
	document.body.innerHTML = '<div id="wm-app"><div id="svelte-global-loader"></div></div>'
	let written = ''
	document.write = (s: string) => {
		written += s
		document.head.insertAdjacentHTML('beforeend', s)
	}
	new Function(headScript)()
	new Function(bodyScript)()
	return { painted: document.getElementById('wm-home-snapshot'), written }
}

function stored(): HomeSnapshot | null {
	const raw = localStorage.getItem(HOME_SNAPSHOT_KEY)
	return raw ? JSON.parse(raw) : null
}

beforeEach(() => {
	localStorage.clear()
	sessionStorage.clear()
	forgetLastSession()
	window.matchMedia = vi.fn(() => ({ matches: false })) as any
	history.replaceState(null, '', '/?workspace=demo')
	setHint('h1')
	rememberUser(user, 'h1')
	renderHome()
})

describe('home snapshot', () => {
	it('paints what was saved, without values, handlers or anything open', () => {
		saveHomeSnapshot('demo', 'demo')
		const { painted, written } = boot('/?workspace=demo')
		expect(painted).not.toBeNull()
		expect(written).toContain('href="/_app/immutable/assets/app.css"')
		const html = painted!.innerHTML
		expect(html).toContain('f/a')
		expect(html).not.toMatch(/typed|draft|menu|onclick|javascript:|id="content"/)
		expect(html).toContain('id="g1"')
	})

	it('hands over to the live page, which keeps the stylesheets', () => {
		saveHomeSnapshot('demo', 'demo')
		boot('/?workspace=demo')
		releaseHomeSnapshot()
		expect(document.getElementById('wm-home-snapshot')).toBeNull()
		expect(document.documentElement.isConnected).toBe(true)
		expect(document.documentElement.hasAttribute('data-wm-home-snapshot')).toBe(false)
		expect(document.getElementById('svelte-global-loader')).not.toBeNull()
		expect(document.querySelector('link[href="/_app/immutable/assets/app.css"]')).not.toBeNull()
	})

	it('keeps a tree the HTML parser would restructure', () => {
		// An anchor in an anchor can only be built through the DOM; parsed as HTML it splits.
		const outer = document.createElement('a')
		outer.href = '/outer'
		outer.appendChild(document.createElement('span')).appendChild(document.createElement('a'))
		const row = document.createElement('div')
		row.className = 'row'
		row.append(outer, Object.assign(document.createElement('div'), { className: 'after' }))
		document.getElementById('content')!.appendChild(row)
		saveHomeSnapshot('demo', 'demo')
		const { painted } = boot('/?workspace=demo')
		expect(painted!.querySelector('.row > .after')).not.toBeNull()
		expect(painted!.querySelector('.row > a span a')).not.toBeNull()
	})

	it('shows nothing under another session and drops the snapshot', () => {
		saveHomeSnapshot('demo', 'demo')
		setHint('h2')
		expect(boot('/?workspace=demo').painted).toBeNull()
		expect(stored()).toBeNull()
	})

	it('shows nothing once the session hint is gone and drops the snapshot', () => {
		saveHomeSnapshot('demo', 'demo')
		setHint(undefined)
		expect(boot('/').painted).toBeNull()
		expect(stored()).toBeNull()
	})

	it('shows nothing for another workspace, page or search, and keeps the snapshot', () => {
		saveHomeSnapshot('demo', 'demo')
		expect(boot('/?workspace=other').painted).toBeNull()
		expect(boot('/runs?workspace=demo').painted).toBeNull()
		expect(boot('/?workspace=demo&search=a').painted).toBeNull()
		localStorage.setItem('workspace', 'other')
		expect(boot('/').painted).toBeNull()
		localStorage.setItem('workspace', 'demo')
		expect(boot('/').painted).not.toBeNull()
		expect(stored()).not.toBeNull()
	})

	it('drops a snapshot of another format, another build, or too old', () => {
		for (const change of [
			{ format: 999 },
			{ build: 'another-build' },
			{ savedAt: Date.now() - 8 * 24 * 3600 * 1000 }
		]) {
			saveHomeSnapshot('demo', 'demo')
			localStorage.setItem(HOME_SNAPSHOT_KEY, JSON.stringify({ ...stored(), ...change }))
			expect(boot('/?workspace=demo').painted).toBeNull()
			expect(stored()).toBeNull()
		}
	})

	it('saves nothing for a session this tab has not confirmed, or without a hint', () => {
		// Another tab logged in: this tab's user is not the session's.
		setHint('h2')
		saveHomeSnapshot('demo', 'demo')
		expect(stored()).toBeNull()
		setHint(undefined)
		rememberUser(user, undefined)
		saveHomeSnapshot('demo', 'demo')
		expect(stored()).toBeNull()
	})
})
