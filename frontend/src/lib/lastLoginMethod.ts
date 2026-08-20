// The login method that last worked on this browser, so the card can put it first and badge it.
// Purely a hint: it is never read for anything but ordering and a label.
export type LastLoginMethod =
	| { kind: 'password' }
	| { kind: 'oauth'; provider: string }
	| { kind: 'saml' }

const CONFIRMED_KEY = 'lastLoginMethod'
// OAuth and SAML leave the page before the outcome is known, so the method is parked here and
// only promoted once a session exists — otherwise an abandoned provider would claim the badge.
const PENDING_KEY = 'lastLoginMethodPending'

function read(key: string): LastLoginMethod | undefined {
	try {
		const raw = localStorage.getItem(key)
		if (!raw) return undefined
		const parsed = JSON.parse(raw)
		if (parsed?.kind === 'password' || parsed?.kind === 'saml') return parsed
		if (parsed?.kind === 'oauth' && typeof parsed.provider === 'string') return parsed
		return undefined
	} catch {
		return undefined
	}
}

function write(key: string, method: LastLoginMethod | undefined) {
	try {
		if (method) localStorage.setItem(key, JSON.stringify(method))
		else localStorage.removeItem(key)
	} catch (e) {
		console.error('Could not record the last login method', e)
	}
}

export function getLastLoginMethod(): LastLoginMethod | undefined {
	return read(CONFIRMED_KEY)
}

export function rememberLoginMethod(method: LastLoginMethod) {
	write(CONFIRMED_KEY, method)
	write(PENDING_KEY, undefined)
}

export function markLoginMethodPending(method: LastLoginMethod) {
	write(PENDING_KEY, method)
}

export function clearPendingLoginMethod() {
	write(PENDING_KEY, undefined)
}

/** Call only where a session is proven: whatever redirect was in flight is what worked. */
export function confirmPendingLoginMethod() {
	const pending = read(PENDING_KEY)
	if (pending) rememberLoginMethod(pending)
}

export function sameLoginMethod(a: LastLoginMethod | undefined, b: LastLoginMethod): boolean {
	if (!a || a.kind !== b.kind) return false
	return a.kind !== 'oauth' || a.provider === (b as { provider: string }).provider
}
