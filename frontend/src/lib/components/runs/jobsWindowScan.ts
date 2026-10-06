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

export function nextWindowWidth(widthMs: number, elapsedMs: number): number {
	const step = Math.min(MAX_STEP, Math.max(1 / MAX_STEP, TARGET_MS / Math.max(elapsedMs, 1)))
	return Math.max(MIN_WIDTH_MS, widthMs * step)
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
	onWindow: (jobs: Job[], scannedTo: string) => void
	patienceMs?: number
}

const SLOW = Symbol('slow')

/**
 * Lists the completed jobs of a time range newest first, one bounded window after the other,
 * which is the same page a single request over the whole range returns, except that no request
 * has to scan more history than the server allows one to. Resolves to whether the range was
 * scanned to its end, false meaning the page filled up first.
 */
export function scanJobWindows(scan: JobWindowScan): CancelablePromise<boolean> {
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
			let first = true
			let width = INITIAL_WIDTH_MS
			let listed = 0
			while (upper > floor) {
				if (onCancel.isCancelled) return false
				const lower = Math.max(floor, upper - width)
				const w: JobWindow = {
					before: first ? (scan.before ?? undefined) : new Date(upper).toISOString(),
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
					if (width <= MIN_WIDTH_MS) {
						throw new Error(
							`Listing the jobs of a single ${MIN_WIDTH_MS / 1000}s window is too slow. Narrow the filters.`
						)
					}
					width = Math.max(MIN_WIDTH_MS, width / MAX_STEP)
					continue
				}
				const completed = res.filter((j) => j.type === 'CompletedJob').length
				scan.onWindow(res, w.after)
				if (completed >= w.limit) return false
				listed += completed
				upper = lower
				first = false
				width = nextWindowWidth(width, Date.now() - startedAt)
			}
			return true
		}

		run().then(resolve, reject)
	})
}
