// Dev-only: freeze recurring polling while nobody is looking at the tab.
//
// `scripts/dev-supervisor.mjs` reclaims a worktree's dev server once traffic stops, but a tab left
// open keeps polling /api forever and would pin it resident. Intervals are cleared while dormant
// and re-armed on the next focus, so returning to the tab costs one extra tick of staleness.

const DEFAULT_DORMANT_AFTER_MS = 5 * 60_000

// Handles are offset so anything we don't recognise in clearInterval can be delegated to the
// native one: clearing an id that never existed is a no-op, mistaking one for ours is not.
const HANDLE_OFFSET = 1_000_000_000

type Registration = {
	handler: TimerHandler
	ms: number
	args: unknown[]
	native: number | undefined
}

const INSTALLED_FLAG = '__wmDevPollingDormancyInstalled'

export function installDevPollingDormancy(): void {
	if (!import.meta.env.DEV || typeof window === 'undefined') return
	// The root layout's body re-runs on HMR, and a second install would bind its "native"
	// setInterval to the already-patched one and stack another set of window listeners. The
	// guard lives on `window` rather than in module scope because hot-replacing this very
	// module resets module state while leaving the previous patch in place.
	const flags = window as unknown as Record<string, boolean>
	if (flags[INSTALLED_FLAG]) return
	flags[INSTALLED_FLAG] = true

	const configured = import.meta.env.VITE_DEV_DORMANT_MS
	const dormantAfterMs = configured ? Number(configured) : DEFAULT_DORMANT_AFTER_MS
	if (!Number.isFinite(dormantAfterMs) || dormantAfterMs <= 0) return

	const nativeSetInterval = window.setInterval.bind(window)
	const nativeClearInterval = window.clearInterval.bind(window)
	const nativeSetTimeout = window.setTimeout.bind(window)
	const nativeClearTimeout = window.clearTimeout.bind(window)

	const registrations = new Map<number, Registration>()
	let nextHandle = HANDLE_OFFSET
	let dormant = false
	let countdown: number | undefined

	window.setInterval = ((handler: TimerHandler, ms?: number, ...args: unknown[]): number => {
		const handle = ++nextHandle
		const registration: Registration = { handler, ms: ms ?? 0, args, native: undefined }
		if (!dormant) {
			registration.native = nativeSetInterval(handler, registration.ms, ...args)
		}
		registrations.set(handle, registration)
		return handle
	}) as typeof window.setInterval

	window.clearInterval = ((handle?: number): void => {
		const registration = handle === undefined ? undefined : registrations.get(handle)
		if (!registration) {
			nativeClearInterval(handle)
			return
		}
		if (registration.native !== undefined) nativeClearInterval(registration.native)
		registrations.delete(handle as number)
	}) as typeof window.clearInterval

	function inactive(): boolean {
		return document.hidden || !document.hasFocus()
	}

	function scheduleDormancy(): void {
		if (dormant || countdown !== undefined) return
		countdown = nativeSetTimeout(() => {
			countdown = undefined
			if (!inactive()) return
			dormant = true
			for (const registration of registrations.values()) {
				if (registration.native === undefined) continue
				nativeClearInterval(registration.native)
				registration.native = undefined
			}
			console.debug(
				`[dev] polling suspended after ${Math.round(dormantAfterMs / 1000)}s inactive ` +
					`(${registrations.size} intervals)`
			)
		}, dormantAfterMs)
	}

	function wake(): void {
		if (countdown !== undefined) {
			nativeClearTimeout(countdown)
			countdown = undefined
		}
		if (!dormant) return
		dormant = false
		// The dev supervisor may have reclaimed the server while we were quiet, and only HTTP
		// restarts it, so put it back up before the app's sockets retry into it.
		void fetch(`${location.origin}/`, { method: 'HEAD', cache: 'no-store' }).catch(() => {})
		for (const registration of registrations.values()) {
			registration.native = nativeSetInterval(
				registration.handler,
				registration.ms,
				...registration.args
			)
		}
		console.debug('[dev] polling resumed')
	}

	window.addEventListener('blur', scheduleDormancy)
	window.addEventListener('focus', wake)
	window.addEventListener('pointerdown', wake)
	window.addEventListener('keydown', wake)
	document.addEventListener('visibilitychange', () => {
		if (document.hidden) scheduleDormancy()
		else wake()
	})

	if (inactive()) scheduleDormancy()
}
