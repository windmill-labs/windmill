import { CancelablePromise, type Job } from '$lib/gen'

// A window is sized so that listing it takes about this long: short enough that the scan keeps
// reporting progress, long enough that a sparse history is not crossed one round trip at a time.
const TARGET_MS = 2000
// Past this the window is dropped and retried narrower. The server keeps running a dropped
// query until its own statement timeout, so it has to be rare: a window grows by at most
// MAX_STEP per round, which keeps a well-sized one under PATIENCE_MS = MAX_STEP * TARGET_MS.
const MAX_STEP = 4
const PATIENCE_MS = MAX_STEP * TARGET_MS
const INITIAL_WIDTH_MS = 60 * 60 * 1000
const MIN_WIDTH_MS = 60 * 1000

function nextWindowWidth(widthMs: number, elapsedMs: number, targetMs: number): number {
	const step = Math.min(MAX_STEP, Math.max(1 / MAX_STEP, targetMs / Math.max(elapsedMs, 1)))
	return Math.max(MIN_WIDTH_MS, widthMs * step)
}

// A window's cost is expected to follow its width. It does not when the database answers it
// from an index that ignores the time bounds (another filter's own index, or no index leading
// with the sort column): every window then costs a pass over the whole history, and scanning
// would multiply the load it exists to bound.
function tooSlow(): Error {
	return new Error(
		`Listing the jobs of a single ${MIN_WIDTH_MS / 1000}s window is too slow. Narrow the filters.`
	)
}

export interface JobWindow {
	/** Upper bound, absent on a window that starts at the newest job. */
	before?: string
	after: string
	limit: number
}

export interface JobWindowScan {
	/** Upper bound of the scan, sent as is on the first window. null starts at the newest job. */
	before: string | null
	/** Lower bound of the scan. null ends it at the oldest job, as given by `oldest`. */
	after: string | null
	pageSize: number
	oldest: () => Promise<string | undefined>
	fetchWindow: (w: JobWindow) => CancelablePromise<Job[]>
	/** `fraction` is the share of the scan's time range listed so far. */
	onWindow: (jobs: Job[], scannedTo: string, fraction: number) => void
	targetMs?: number
	patienceMs?: number
}

const SLOW = Symbol('slow')

// Both bounds of a window are inclusive on the server, whose timestamps have microsecond
// precision. A window therefore ends one microsecond before the previous one started: sharing
// the bound would list a job created exactly on it twice, and count it twice toward the page.
function justBefore(ms: number): string {
	return new Date(ms - 1).toISOString().replace('Z', '999Z')
}

/**
 * Lists the completed jobs of a time range newest first, one bounded window after the other,
 * which is the same page a single request over the whole range returns, except that no request
 * has to scan more history than the server allows one to. Resolves to whether the range was
 * scanned to its end, false meaning the page filled up first.
 */
export function scanJobWindows(scan: JobWindowScan): CancelablePromise<boolean> {
	const targetMs = scan.targetMs ?? TARGET_MS
	const patienceMs = scan.patienceMs ?? PATIENCE_MS
	return new CancelablePromise<boolean>((resolve, reject, onCancel) => {
		let current: CancelablePromise<Job[]> | undefined
		onCancel(() => current?.cancel())

		async function fetchWithPatience(w: JobWindow): Promise<Job[] | typeof SLOW> {
			const request = scan.fetchWindow(w)
			current = request
			let timer: ReturnType<typeof setTimeout> | undefined
			const slow = new Promise<typeof SLOW>((r) => (timer = setTimeout(() => r(SLOW), patienceMs)))
			try {
				const res = await Promise.race([request, slow])
				if (res === SLOW) {
					request.catch(() => {})
					request.cancel()
				}
				return res
			} finally {
				clearTimeout(timer)
			}
		}

		async function run(): Promise<boolean> {
			const floorTs = scan.after ?? (await scan.oldest())
			if (!floorTs) return true
			const floor = new Date(floorTs).getTime()
			let upper = scan.before ? new Date(scan.before).getTime() : Date.now()
			const start = upper
			let first = true
			let width = INITIAL_WIDTH_MS
			let listed = 0
			// The first window is listed even when the scan starts at its floor: the bounds are
			// inclusive, and jobs sharing the cursor's timestamp may still be missing from the page.
			while (first || upper > floor) {
				if (onCancel.isCancelled) return false
				const lower = Math.min(upper, Math.max(floor, upper - width))
				const w: JobWindow = {
					before: first ? (scan.before ?? undefined) : justBefore(upper),
					after: new Date(lower).toISOString(),
					limit: scan.pageSize - listed
				}
				const startedAt = Date.now()
				let res: Job[] | typeof SLOW
				try {
					res = await fetchWithPatience(w)
				} catch (e) {
					// A window the server refused to finish is retried narrower like a slow one.
					if (onCancel.isCancelled || width <= MIN_WIDTH_MS) throw e
					res = SLOW
				}
				if (res === SLOW) {
					if (width <= MIN_WIDTH_MS) throw tooSlow()
					// Well below what the elapsed time suggests: each dropped window is one more
					// statement left running on the server, so the retry has to land.
					width = Math.max(MIN_WIDTH_MS, width / (MAX_STEP * MAX_STEP))
					continue
				}
				const completed = res.filter((j) => j.type === 'CompletedJob').length
				scan.onWindow(res, w.after, start > floor ? (start - lower) / (start - floor) : 1)
				if (completed >= w.limit) return false
				listed += completed
				upper = lower
				first = false
				const elapsed = Date.now() - startedAt
				if (width <= MIN_WIDTH_MS && elapsed > targetMs && upper > floor) throw tooSlow()
				width = nextWindowWidth(width, elapsed, targetMs)
			}
			return true
		}

		run().then(resolve, reject)
	})
}
