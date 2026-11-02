import type { UserExt } from '$lib/stores'

// The user `whoami` last confirmed, so a reload can render as them before `whoami` answers.
// Bound to the session the browser holds: the session cookie is HttpOnly, so the backend
// mirrors a hint of it into a readable cookie (`session_hint.rs`), and a remembered user is
// only used while that hint still matches. A session that ended or changed since, by logout,
// expiry or another login, leaves no hint or another one, so the next person on the browser
// is never rendered as the previous one.

const KEY = 'wm-last-session'

interface LastSession {
	hint: string
	user: UserExt
}

export function readSessionHint(cookies: string = document.cookie): string | undefined {
	for (const part of cookies.split(/;\s*/)) {
		const eq = part.indexOf('=')
		if (eq > 0 && part.slice(0, eq) === __WM_SESSION_HINT__) return part.slice(eq + 1) || undefined
	}
	return undefined
}

/** The user to render as in `workspace` until `whoami` answers, if the session is still theirs. */
export function optimisticUser(workspace: string): UserExt | undefined {
	try {
		const hint = readSessionHint()
		const raw = localStorage.getItem(KEY)
		if (!hint || !raw) return undefined
		const last = JSON.parse(raw) as LastSession
		return last.hint === hint && last.user?.workspace_id === workspace ? last.user : undefined
	} catch {
		return undefined
	}
}

export function rememberUser(user: UserExt): void {
	try {
		const hint = readSessionHint()
		if (hint) localStorage.setItem(KEY, JSON.stringify({ hint, user } satisfies LastSession))
		else localStorage.removeItem(KEY)
	} catch {}
}

export function forgetLastSession(): void {
	try {
		localStorage.removeItem(KEY)
	} catch {}
}
