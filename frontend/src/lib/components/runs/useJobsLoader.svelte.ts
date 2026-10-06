import { onDestroy, onMount, untrack } from 'svelte'
import {
	JobService,
	type Job,
	type CompletedJob,
	type ExtendedJobs,
	ConcurrencyGroupsService,
	type ObscuredJob,
	CancelablePromise,
	CancelError
} from '$lib/gen'

import { sendUserToast } from '$lib/toast'

import { tweened, type Tweened } from 'svelte/motion'
import { subtractDaysFromDateString } from '$lib/utils'
import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'
import type { Timeframe } from './timeframes'
import { allowWildcards as _allowWildcards, type RunsFilterInstance } from './runsFilter'
import { scanJobWindows } from './jobsWindowScan'

// windmill_common::utils::MAX_PER_PAGE: the server silently caps per_page at this value
const MAX_PER_PAGE = 10000

// How long a listing over an open-ended range may run before it is dropped for a scan by time
// windows. A filter matching fewer jobs than the page holds walks the whole history to find that
// out, which the server stops after its statement timeout, with nothing to show for it.
const WINDOW_SCAN_AFTER_MS = 4000

const EPOCH = new Date(0).toISOString()

interface JobsQuery {
	completedBefore?: string | null
	completedAfter?: string | null
	createdBefore?: string
	createdAfter?: string
	createdAfterQueue?: string
	perPage?: number
}

export function computeJobKinds(jobKindsCat: string | null): string {
	if (jobKindsCat == 'all') {
		return ''
	} else if (jobKindsCat == 'dependencies') {
		let kinds: CompletedJob['job_kind'][] = ['dependencies', 'flowdependencies', 'appdependencies']
		return kinds.join(',')
	} else if (jobKindsCat == 'previews') {
		let kinds: CompletedJob['job_kind'][] = ['preview', 'flowpreview']
		return kinds.join(',')
	} else if (jobKindsCat == 'deploymentcallbacks') {
		let kinds: CompletedJob['job_kind'][] = ['deploymentcallback']
		return kinds.join(',')
	} else {
		// Default mirrors the explicit 'runs' category — top-level scripts, flows,
		// and single-step flows. flowscript/flownode/appscript are intermediate
		// flow children with non-null parent_job, and the loader pairs this with
		// hasNullParent: true, so they would never match here anyway.
		let kinds: CompletedJob['job_kind'][] = ['script', 'flow', 'singlestepflow']
		return kinds.join(',')
	}
}

export interface UseJobLoaderArgs {
	currentWorkspace: string
	filters?: Partial<RunsFilterInstance>
	timeframe?: Timeframe
	jobKinds?: string
	autoRefresh?: boolean
	argError?: string
	resultError?: string
	refreshRate?: number
	syncQueuedRunsCount?: boolean
	skip?: boolean
	lookback?: number
	perPage?: number
	excludesEntrypointOverride?: boolean
}

export function useJobsLoader(args: () => UseJobLoaderArgs) {
	let _args = $derived(args())

	let currentWorkspace = $derived(_args.currentWorkspace)
	let filters = $derived(_args.filters)
	let jobKinds = $derived(_args.jobKinds)
	let autoRefresh = $derived(_args.autoRefresh ?? true)
	let argError = $derived(_args.argError ?? '')
	let resultError = $derived(_args.resultError ?? '')
	let refreshRate = $derived(_args.refreshRate ?? 5000)
	let syncQueuedRunsCount = $derived(_args.syncQueuedRunsCount ?? true)
	let lookback = $derived(_args.lookback ?? 0)
	let timeframe = $derived(_args?.timeframe)
	let perPage = $derived(_args?.perPage ?? 1000)
	let excludesEntrypointOverride = $derived(_args.excludesEntrypointOverride ?? false)

	let label = $derived(filters?.label ?? null)
	let worker = $derived(filters?.worker ?? null)
	let success = $derived(filters?.status ?? null)
	let isQueueOnly = $derived(success == 'running' || success == 'suspended' || success == 'waiting')
	let showSkipped = $derived(filters?.show_skipped ?? false)
	let resolutionFilter = $derived(filters?.resolved ?? 'all')
	let showSchedules = $derived(!filters?.job_trigger_kind?.includes('!schedule'))
	let showFutureJobs = $derived(filters?.show_future_jobs ?? true)
	let resultFilter = $derived(filters?.result)
	let jobTriggerKind = $derived(filters?.job_trigger_kind ?? null)
	let schedulePath = $derived(filters?.schedule_path ?? null)
	let jobKindsCat = $derived(filters?.job_kinds ?? null)
	let allWorkspaces = $derived(filters?.all_workspaces ?? false)
	let allowWildcards = $derived(_allowWildcards(filters))
	let concurrencyKey = $derived(filters?.concurrency_key)
	let tag = $derived(filters?.tag)
	let user = $derived(filters?.user)
	let folder = $derived(filters?.folder)
	let path = $derived(filters?.path)
	let argFilter = $derived(filters?.arg)
	let broadFilter = $derived(filters?._default_ || undefined)

	let queue_count: Tweened<number> | undefined = $state()
	let suspended_count: Tweened<number> | undefined = $state()
	let loading = $state(false)
	let loadingExtra = $state(false)
	let lastFetchWentToEnd = $state(true)
	let batchProgress = $state<{ loaded: number; total: number } | null>(null)
	let scanProgress = $state<{ scannedTo: string } | null>(null)
	let activeScan: CancelablePromise<boolean> | undefined
	// Queue-only views list no completed job, and concurrency-key views go through another endpoint.
	let canScanWindows = $derived(!isQueueOnly && (concurrencyKey == null || concurrencyKey === ''))

	let completedJobs: CompletedJob[] | undefined = $state()
	let externalJobs: Job[] | undefined = $state()
	let extendedJobs: ExtendedJobs | undefined = $state()
	let jobs: Job[] | undefined = $state()

	let intervalId: ReturnType<typeof setInterval> | undefined = $state()
	let sync = true
	let paramChangeTimeout: ReturnType<typeof setTimeout> | undefined
	let paramChangePromise: CancelablePromise<void> | undefined
	let slowStreamIntervalId: ReturnType<typeof setInterval> | undefined
	let currentBatchSize = $state<number | null>(null)

	function onParamChanges() {
		resetJobs()
		slowStreamIntervalId = setInterval(() => {
			if (scanProgress) return
			sendUserToast(
				'Loading is taking a long time...',
				'warning',
				[{ label: 'Stop loading', callback: () => stopBatchLoading() }],
				undefined,
				8000
			)
		}, 15000)
		let promise = loadJobsIntern(true)
		if (perPage > 25 && !canScanWindows) {
			promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
				const noStartDate = timeframe?.computeMinMax().minTs == null
				sendUserToast(
					(success == 'failure' || success == 'canceled') && noStartDate
						? `Loading ${success == 'failure' ? 'failed' : 'canceled'} jobs with no start date scans the full job history. Set a time range to speed it up.`
						: 'Loading jobs is taking longer than expected...',
					'warning',
					[{ label: 'Stream by batches of 25', callback: () => restreamWithSmallBatches() }]
				)
			})
		}
		promise = CancelablePromiseUtils.finallyDo(promise, () => {
			if (slowStreamIntervalId) {
				clearInterval(slowStreamIntervalId)
				slowStreamIntervalId = undefined
			}
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) {
				return CancelablePromiseUtils.pure<void>(undefined)
			}
			return CancelablePromiseUtils.err(e)
		})
		return promise
	}

	function restreamWithBatchSize(size: number) {
		paramChangePromise?.cancel()
		resetJobs()
		slowStreamIntervalId = setInterval(() => {
			if (scanProgress) return
			sendUserToast(
				'Loading is taking a long time...',
				'warning',
				[{ label: 'Stop loading', callback: () => stopBatchLoading() }],
				undefined,
				8000
			)
		}, 15000)
		paramChangePromise = loadJobsIntern(false, size)
		paramChangePromise = CancelablePromiseUtils.finallyDo(paramChangePromise, () => {
			if (slowStreamIntervalId) {
				clearInterval(slowStreamIntervalId)
				slowStreamIntervalId = undefined
			}
		})
		paramChangePromise = CancelablePromiseUtils.catchErr(paramChangePromise, (e) => {
			if (e instanceof CancelError) {
				return CancelablePromiseUtils.pure<void>(undefined)
			}
			return CancelablePromiseUtils.err(e)
		})
	}

	function restreamWithSmallBatches() {
		restreamWithBatchSize(25)
	}

	function restreamWithSingleItems() {
		restreamWithBatchSize(1)
	}

	let loadingFetch = false

	async function loadExtraJobs(): Promise<void> {
		loading = true
		loadingExtra = true
		const batchSize = Math.min(perPage, 1000)
		try {
			await loadExtraJobsBatch(batchSize)
		} catch (e) {
			if (!(e instanceof CancelError)) throw e
		} finally {
			loadingExtra = false
		}
	}

	// Mirrors when list_completed_jobs_query sorts by completed_at. A created_at cursor does not
	// bound that index scan, so each batch would rescan from the newest job, and skip jobs created
	// after the cursor but completed before it.
	function sortsByCompletedAt(minTs: string | null, maxTs: string | null): boolean {
		return minTs != null || maxTs != null || success == 'failure' || success == 'canceled'
	}

	function loadExtraJobsBatch(batchSize: number): CancelablePromise<void> {
		if (!jobs || jobs.length === 0) {
			lastFetchWentToEnd = true
			return CancelablePromiseUtils.pure<void>(undefined as void)
		}
		const { minTs, maxTs } = timeframe?.computeMinMax() ?? { minTs: null, maxTs: null }
		const byCompletedAt =
			jobs[jobs.length - 1].type === 'CompletedJob' && sortsByCompletedAt(minTs, maxTs)
		const sortKey = (j: Job) =>
			byCompletedAt ? (j.type === 'CompletedJob' ? j.completed_at : undefined) : j.created_at
		const cursorTs = sortKey(jobs[jobs.length - 1])
		if (!cursorTs) {
			lastFetchWentToEnd = true
			return CancelablePromiseUtils.pure<void>(undefined as void)
		}
		// Inclusive cursor at the API's microsecond precision: jobs sharing the boundary timestamp (e.g.
		// a bulk cancel) are refetched rather than skipped, and the page grows by those already listed,
		// up to the server's MAX_PER_PAGE. Once the listed part of the group fills that cap, the cursor
		// steps just below the group, dropping its remainder instead of ending the list early.
		const tied = jobs.filter((j) => sortKey(j) === cursorTs).length
		const stepOver = tied >= MAX_PER_PAGE
		const cursor = stepOver ? new Date(new Date(cursorTs).getTime() - 1).toISOString() : cursorTs
		const pageSize = stepOver ? batchSize : Math.min(batchSize + tied, MAX_PER_PAGE)
		return CancelablePromiseUtils.map(
			fetchPage(
				{
					byCompletedAt,
					before: cursor,
					after: minTs,
					pageSize,
					query: byCompletedAt
						? { completedBefore: cursor, completedAfter: minTs, perPage: pageSize }
						: { completedAfter: minTs, createdBefore: cursor, perPage: pageSize }
				},
				(olderJobs) => {
					jobs = updateWithNewJobs(olderJobs, jobs ?? [])
					if (extendedJobs) {
						extendedJobs.jobs = jobs ?? []
						extendedJobs = extendedJobs
					}
					computeCompletedJobs()
				}
			),
			(wentToEnd) => {
				lastFetchWentToEnd = wentToEnd
				loading = false
			}
		)
	}

	// Lists one page of jobs older than `before`, handing them to `onJobs` as they arrive, and
	// resolves to whether the listing reached the end of the range. `query` is the whole page in
	// one request; when that is too slow the page is listed by time windows instead.
	function fetchPage(
		page: {
			byCompletedAt: boolean
			before: string | null
			after: string | null
			pageSize: number
			withQueue?: boolean
			query: JobsQuery
		},
		onJobs: (jobs: Job[]) => void
	): CancelablePromise<boolean> {
		const single = fetchJobs(page.query)
		if (!canScanWindows || _args.skip) {
			return CancelablePromiseUtils.map(single, (res) => {
				onJobs(res)
				return res.length < page.pageSize
			})
		}
		return new CancelablePromise<boolean>((resolve, reject, onCancel) => {
			let scan: CancelablePromise<boolean> | undefined
			onCancel(() => {
				clearTimeout(timer)
				single.cancel()
				scan?.cancel()
			})
			const timer = setTimeout(() => {
				single.cancel()
				scan = scanWindows(page, onJobs)
				scan.then(resolve, reject)
			}, WINDOW_SCAN_AFTER_MS)
			single.then(
				(res) => {
					clearTimeout(timer)
					onJobs(res)
					resolve(res.length < page.pageSize)
				},
				(e) => {
					if (!scan) {
						clearTimeout(timer)
						reject(e)
					}
				}
			)
		})
	}

	function scanWindows(
		page: {
			byCompletedAt: boolean
			before: string | null
			after: string | null
			pageSize: number
			withQueue?: boolean
		},
		onJobs: (jobs: Job[]) => void
	): CancelablePromise<boolean> {
		const workspace = currentWorkspace
		const scan = scanJobWindows({
			before: page.before,
			after: page.after,
			pageSize: page.pageSize,
			oldest: async () =>
				(await JobService.getOldestJob({ workspace, allWorkspaces: allWorkspaces || undefined }))
					.created_at,
			fetchWindow: (w) => {
				// Only the first window of a listing that started at the newest job carries the queue.
				// A created_before excludes it from the others, and bounds no completed job out of a
				// completed_at window since a job is created before it completes.
				const withQueue = page.withQueue && w.before == page.before
				return listJobs(
					page.byCompletedAt
						? {
								completedBefore: w.before,
								completedAfter: w.after,
								createdBefore: withQueue ? undefined : w.before,
								createdAfterQueue: page.after ?? undefined,
								perPage: w.limit
							}
						: {
								createdBefore: w.before,
								createdAfter: w.after,
								// created_after would bound the queue too, hiding older jobs still in it
								createdAfterQueue: withQueue ? EPOCH : undefined,
								perPage: w.limit
							}
				)
			},
			onWindow: (res, scannedTo) => {
				if (activeScan !== scan) return
				scanProgress = { scannedTo }
				onJobs(res)
			}
		})
		activeScan?.cancel()
		activeScan = scan
		scanProgress = { scannedTo: page.before ?? new Date().toISOString() }
		onJobs([])
		const done = () => {
			if (activeScan === scan) {
				activeScan = undefined
				scanProgress = null
			}
		}
		scan.then(done, done)
		return CancelablePromiseUtils.catchErr(scan, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.err(e)
			sendUserToast(`Could not load jobs: ${e.body ?? e.message}`, true)
			console.error(e)
			return CancelablePromiseUtils.pure(false)
		})
	}

	function fetchJobs(q: JobsQuery): CancelablePromise<Job[]> {
		if (_args.skip) return CancelablePromiseUtils.pure<Job[]>([])
		return CancelablePromiseUtils.catchErr(listJobs(q), (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.err(e)
			sendUserToast(`Could not load jobs: ${e.body ?? e.message}`, true)
			console.error(e)
			return CancelablePromiseUtils.pure<Job[]>([])
		})
	}

	function listJobs({
		completedBefore,
		completedAfter,
		createdBefore,
		createdAfter,
		createdAfterQueue,
		perPage: perPageOverride
	}: JobsQuery): CancelablePromise<Job[]> {
		loadingFetch = true
		let scriptPathStart = folder == null || folder === '' ? undefined : `f/${folder}/`
		let scriptPathExact = path == null || path === '' ? undefined : path
		let isCompletedOnly = success == 'success' || success == 'failure' || success == 'canceled'
		let promise = JobService.listJobs({
			workspace: currentWorkspace,
			createdBefore,
			createdAfter,
			completedBefore: isQueueOnly ? undefined : (completedBefore ?? undefined),
			completedAfter: isQueueOnly ? undefined : (completedAfter ?? undefined),
			createdBeforeQueue: isQueueOnly ? (completedBefore ?? undefined) : undefined,
			createdAfterQueue: isCompletedOnly
				? undefined
				: isQueueOnly
					? (completedAfter ?? createdAfterQueue)
					: createdAfterQueue,
			schedulePath: schedulePath ?? undefined,
			scriptPathExact,
			createdBy: user == null || user === '' ? undefined : user,
			scriptPathStart: scriptPathStart,
			jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
			success: success == 'success' ? true : undefined,
			status: success == 'failure' || success == 'canceled' ? success : undefined,
			running:
				success == 'running' || success == 'suspended'
					? true
					: success == 'waiting'
						? false
						: undefined,
			isSkipped: showSkipped ? undefined : false,
			resolved:
				resolutionFilter === 'resolved'
					? true
					: resolutionFilter === 'unresolved'
						? false
						: undefined,
			// isFlowStep: jobKindsCat != 'all' ? false : undefined,
			hasNullParent: jobKindsCat != 'all' ? true : undefined,
			label: label == null || label === '' ? undefined : label,
			tag: tag == null || tag === '' ? undefined : tag,
			worker: worker == null || worker === '' ? undefined : worker,
			isNotSchedule: showSchedules == false ? true : undefined,
			suspended: success == 'waiting' ? false : success == 'suspended' ? true : undefined,
			scheduledForBeforeNow:
				showFutureJobs == false || success == 'waiting' || success == 'suspended'
					? true
					: undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			triggerKind: jobTriggerKind ?? undefined,
			allWorkspaces: allWorkspaces ? true : undefined,
			perPage: perPageOverride ?? perPage,
			allowWildcards: allowWildcards ? true : undefined,
			broadFilter,
			excludesEntrypointOverride: excludesEntrypointOverride ? true : undefined
		})
		const settled = () => {
			loadingFetch = false
		}
		promise.then(settled, settled)
		return promise
	}

	function fetchExtendedJobs(
		concurrencyKey: string | null,
		createdBeforeQueue: string | null,
		completedAfter: string | null
	): CancelablePromise<ExtendedJobs> {
		if (_args.skip)
			return CancelablePromiseUtils.pure<ExtendedJobs>({ jobs: [], obscured_jobs: [] })
		loadingFetch = true
		let promise = ConcurrencyGroupsService.listExtendedJobs({
			rowLimit: perPage,
			concurrencyKey: concurrencyKey == null || concurrencyKey == '' ? undefined : concurrencyKey,
			workspace: currentWorkspace,
			completedAfter: completedAfter ?? undefined,
			createdBeforeQueue: createdBeforeQueue ?? undefined,
			// createdOrStartedBefore: startedBefore,
			// createdOrStartedAfter: startedAfter,
			// createdOrStartedAfterCompletedJobs: startedAfterCompletedJobs,
			schedulePath: schedulePath ?? undefined,
			scriptPathExact: path == null || path === '' ? undefined : path,
			createdBy: user == null || user === '' ? undefined : user,
			scriptPathStart: folder == null || folder === '' ? undefined : `f/${folder}/`,
			jobKinds: jobKindsCat == 'all' || jobKinds == '' ? undefined : jobKinds,
			success: success == 'success' ? true : undefined,
			status: success == 'failure' || success == 'canceled' ? success : undefined,
			running: success == 'running' ? true : undefined,
			isSkipped: showSkipped ? undefined : false,
			resolved:
				resolutionFilter === 'resolved'
					? true
					: resolutionFilter === 'unresolved'
						? false
						: undefined,
			isFlowStep: jobKindsCat != 'all' ? false : undefined,
			label: label == null || label === '' ? undefined : label,
			tag: tag == null || tag === '' ? undefined : tag,
			isNotSchedule: showSchedules == false ? true : undefined,
			scheduledForBeforeNow: showFutureJobs == false ? true : undefined,
			args:
				argFilter && argFilter != '{}' && argFilter != '' && argError == '' ? argFilter : undefined,
			result:
				resultFilter && resultFilter != '{}' && resultFilter != '' && resultError == ''
					? resultFilter
					: undefined,
			triggerKind: jobTriggerKind ?? undefined,
			allWorkspaces: allWorkspaces ? true : undefined,
			perPage,
			allowWildcards
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			sendUserToast('There was an issue loading jobs, see browser console for more details', true)
			console.error(e)
			return CancelablePromiseUtils.pure({ jobs: [], obscured_jobs: [] } as ExtendedJobs)
		})
		promise = CancelablePromiseUtils.pipe(promise, () => {
			loadingFetch = false
		})
		return promise
	}

	async function loadJobs(reset: boolean, shouldGetCount?: boolean): Promise<void> {
		if (reset) resetJobs()
		await loadJobsIntern(shouldGetCount)
	}

	function stopBatchLoading(): void {
		paramChangePromise?.cancel()
		activeScan?.cancel()
		if (slowStreamIntervalId) {
			clearInterval(slowStreamIntervalId)
			slowStreamIntervalId = undefined
		}
		batchProgress = null
		currentBatchSize = null
		loading = false
	}

	function resetJobs() {
		// A scan started by "load more" outlives the listing it extends unless stopped here.
		activeScan?.cancel()
		jobs = undefined
		completedJobs = undefined
		externalJobs = undefined
		extendedJobs = undefined
		lastFetchWentToEnd = false
		batchProgress = null
		currentBatchSize = null
		if (slowStreamIntervalId) {
			clearInterval(slowStreamIntervalId)
			slowStreamIntervalId = undefined
		}
		intervalId && clearInterval(intervalId)
		intervalId = setInterval(syncer, refreshRate)
	}
	function loadJobsIntern(
		shouldGetCount?: boolean,
		overrideBatchSize?: number
	): CancelablePromise<void> {
		const { minTs, maxTs } = timeframe?.computeMinMax() ?? { minTs: null, maxTs: null }
		listLoadedAt = new Date(Date.now() - 5 * 60_000).toISOString()
		if (shouldGetCount) {
			getCount()
		}
		loading = true
		// Extend MinTs to fetch jobs mefore minTs and show a correct concurrency graph
		// TODO: when an ended_at column is created on the completed_job table,
		// lookback won't be needed anymore (just filter ended_at > minTs instead
		const extendedMinTs = subtractDaysFromDateString(minTs, lookback)

		if (concurrencyKey == null || concurrencyKey === '') {
			const batchSize = overrideBatchSize ?? Math.min(perPage, 1000)
			const isBatched = perPage > batchSize
			if (isBatched) {
				batchProgress = { loaded: 0, total: perPage }
			}
			currentBatchSize = batchSize
			// The windowed scan streams a slow listing by itself, whatever the batch size.
			let slowBatchToastShown = canScanWindows
			let firstJobs: Job[] = []
			let firstFetchPromise = fetchPage(
				{
					byCompletedAt: sortsByCompletedAt(minTs, maxTs),
					before: maxTs,
					after: extendedMinTs ?? null,
					pageSize: batchSize,
					withQueue: true,
					query: {
						completedBefore: maxTs,
						completedAfter: extendedMinTs ?? null,
						createdAfterQueue: extendedMinTs,
						perPage: batchSize
					}
				},
				(newJobs) => {
					firstJobs = updateWithNewJobs(newJobs, firstJobs)
					extendedJobs = { jobs: firstJobs, obscured_jobs: [] } as ExtendedJobs

					// Filter on minTs here and not in the backend
					// to get enough data for the concurrency graph
					jobs = sortMinDate(minTs, firstJobs)
					externalJobs = []
					computeCompletedJobs()
				}
			)
			if (isBatched && batchSize > 1 && !slowBatchToastShown) {
				firstFetchPromise = CancelablePromiseUtils.onTimeout(firstFetchPromise, 4000, () => {
					if (!slowBatchToastShown) {
						slowBatchToastShown = true
						sendUserToast(
							`Streaming by batches of ${batchSize} is slow, try loading one at a time`,
							'warning',
							[{ label: 'Stream 1 by 1', callback: () => restreamWithSingleItems() }]
						)
					}
				})
			}
			let p = CancelablePromiseUtils.map(firstFetchPromise, (wentToEnd) => {
				lastFetchWentToEnd = wentToEnd
				loading = false
				if (isBatched) {
					batchProgress = { loaded: jobs?.length ?? 0, total: perPage }
				}
			})
			if (isBatched) {
				const numExtraBatches = Math.ceil(perPage / batchSize) - 1
				for (let i = 0; i < numExtraBatches; i++) {
					p = CancelablePromiseUtils.then(p, (): CancelablePromise<void> => {
						if (lastFetchWentToEnd || !jobs)
							return CancelablePromiseUtils.pure<void>(undefined as void)
						loading = true
						let batchPromise = CancelablePromiseUtils.pipe(loadExtraJobsBatch(batchSize), () => {
							batchProgress = { loaded: jobs?.length ?? 0, total: perPage }
						})
						if (batchSize > 1 && !slowBatchToastShown) {
							batchPromise = CancelablePromiseUtils.onTimeout(batchPromise, 4000, () => {
								if (!slowBatchToastShown) {
									slowBatchToastShown = true
									sendUserToast(
										`Streaming by batches of ${batchSize} is slow, try loading one at a time`,
										'warning',
										[{ label: 'Stream 1 by 1', callback: () => restreamWithSingleItems() }]
									)
								}
							})
						}
						return batchPromise
					})
				}
			}
			p = CancelablePromiseUtils.pipe(p, () => {
				batchProgress = null
				currentBatchSize = null
			})
			return p
		} else {
			return CancelablePromiseUtils.map(
				fetchExtendedJobs(concurrencyKey, maxTs, extendedMinTs ?? null),
				(newExtendedJobs) => {
					extendedJobs = newExtendedJobs
					const newJobs = newExtendedJobs.jobs
					const newExternalJobs = newExtendedJobs.obscured_jobs

					// Filter on minTs here and not in the backend
					// to get enough data for the concurrency graph
					if (minTs != undefined) {
						const minDate = new Date(minTs)
						jobs = newJobs.filter((x) =>
							x.started_at
								? new Date(x.started_at) > minDate
								: x.created_at
									? new Date(x.created_at) > minDate
									: false
						)
						externalJobs = computeExternalJobs(
							newExternalJobs.filter((x) => x.started_at && new Date(x.started_at) > minDate)
						)
					} else {
						jobs = newJobs
						externalJobs = computeExternalJobs(newExternalJobs)
					}
					computeCompletedJobs()
					lastFetchWentToEnd = newJobs.length < perPage
					loading = false
				}
			)
		}
	}

	async function getCount() {
		if (_args.skip) return
		const { database_length, suspended } = await JobService.getQueueCount({
			workspace: currentWorkspace,
			allWorkspaces
		})

		if (queue_count) {
			queue_count.set(database_length)
		} else {
			queue_count = tweened(database_length, { duration: 1000 })
		}
		if (suspended_count) {
			suspended_count.set(suspended ?? 0)
		} else {
			suspended_count = tweened(suspended ?? 0, { duration: 1000 })
		}
	}

	let lastQueueTs: string | undefined = undefined
	let listLoadedAt: string | null = null

	async function syncer() {
		if (loadingFetch) {
			return
		}
		if (timeframe?.type === 'manual') {
			return
		}
		if (sync) {
			if (syncQueuedRunsCount) {
				getCount()
			}

			const { minTs, maxTs } = timeframe?.computeMinMax() ?? { minTs: null, maxTs: null }
			if (jobs) {
				if (success == 'running') {
					loadJobsIntern(false)
				} else {
					let minQueueCreatedAt: string | undefined = undefined
					let completedTs: string | null = null

					let cursor = 0

					if (minTs == undefined) {
						while (cursor < jobs.length) {
							const cjob = jobs[cursor]
							if (cjob.type == 'QueuedJob') {
								minQueueCreatedAt = cjob.created_at
							} else if (cjob.type == 'CompletedJob' && completedTs == undefined) {
								completedTs = new Date(cjob.completed_at!).toISOString()
							}
							cursor++
						}
					}

					let queueTs: string | undefined
					if (minQueueCreatedAt) {
						const queueTs = new Date(minQueueCreatedAt).toISOString()
						lastQueueTs = queueTs
					} else {
						queueTs = lastQueueTs
					}

					loading = true
					let newJobs: Job[]
					if (concurrencyKey == null || concurrencyKey === '') {
						// With no completed job to anchor on, each refresh would repeat the initial
						// unbounded scan of completed jobs, possibly the one that just timed out. Not for
						// queue-only views: fetchJobs turns this into the queue's created_at bound, hiding
						// older jobs that suspend or come due. The margin absorbs browser/database skew.
						newJobs = await fetchJobs({
							completedBefore: maxTs,
							completedAfter: minTs ?? completedTs ?? (isQueueOnly ? null : listLoadedAt),
							createdAfterQueue: queueTs
						})
					} else {
						// Obscured jobs have no ids, so we have to do the full request
						extendedJobs = await fetchExtendedJobs(concurrencyKey, maxTs, minTs ?? completedTs)
						externalJobs = computeExternalJobs(extendedJobs.obscured_jobs)

						// Filter on minTs here and not in the backend
						// to get enough data for the concurrency graph
						newJobs = sortMinDate(minTs ?? completedTs, extendedJobs.jobs)
					}
					if (newJobs && newJobs.length > 0 && jobs) {
						jobs = updateWithNewJobs(jobs, newJobs)
						if (concurrencyKey == null || concurrencyKey === '') {
							if (!extendedJobs) {
								extendedJobs = { jobs: jobs, obscured_jobs: [] } as ExtendedJobs
							} else {
								extendedJobs.jobs = updateWithNewJobs(extendedJobs.jobs, newJobs)
								extendedJobs = extendedJobs
							}
							externalJobs = []
						}
						computeCompletedJobs()
					}
					loading = false
				}
			}
		}
	}

	function updateWithNewJobs(jobs: Job[], newJobs: Job[]) {
		const oldJobs = jobs?.map((x) => x.id)
		let ret = newJobs.filter((x) => !oldJobs.includes(x.id)).concat(jobs)
		newJobs
			.filter((x) => oldJobs.includes(x.id))
			.forEach((x) => (ret![ret?.findIndex((y) => y.id == x.id)!] = x))
		return ret
	}

	function sortMinDate(minTs: string | null, jobs: Job[]) {
		if (minTs) {
			const minDate = new Date(minTs)
			return jobs.filter((x) =>
				x.started_at
					? new Date(x.started_at) > minDate
					: x.created_at
						? new Date(x.created_at) > minDate
						: false
			)
		} else {
			return jobs
		}
	}

	function computeCompletedJobs() {
		completedJobs =
			jobs?.filter((x) => x.type == 'CompletedJob').map((x) => x as CompletedJob) ?? []
	}

	function onVisibilityChange() {
		if (document.hidden) {
			sync = false
		} else {
			sync = true
		}
	}

	function computeExternalJobs(obscuredJobs: ObscuredJob[]) {
		return obscuredJobs.map(
			(x) =>
				({
					type: x.typ,
					started_at: x.started_at,
					running: x.started_at != undefined,
					id: '-',
					script_path: '-',
					created_by: '-',
					created_at: '-',
					success: false,
					canceled: false,
					is_flow_step: false,
					is_skipped: false,
					visible_to_owner: false,
					email: '-',
					permissioned_as: '-',
					tag: '-',
					job_kind: 'script',
					duration_ms: x.duration_ms
				}) as Job
		)
	}

	onMount(() => {
		document.addEventListener('visibilitychange', onVisibilityChange)

		return () => {
			window.removeEventListener('visibilitychange', onVisibilityChange)
		}
	})

	onDestroy(() => {
		sync = false
		if (intervalId) {
			clearInterval(intervalId)
		}
		if (slowStreamIntervalId) {
			clearInterval(slowStreamIntervalId)
			slowStreamIntervalId = undefined
		}
		paramChangePromise?.cancel()
		activeScan?.cancel()
	})
	$effect(() => {
		Object.keys(filters ?? {}).map((k) => filters?.[k as keyof RunsFilterInstance])
		currentWorkspace
		lookback
		perPage
		showSchedules
		showFutureJobs
		clearTimeout(paramChangeTimeout)
		paramChangePromise?.cancel()
		paramChangeTimeout = setTimeout(() => {
			paramChangePromise = untrack(() => onParamChanges())
		}, 0)
		return () => {
			clearTimeout(paramChangeTimeout)
			paramChangePromise?.cancel()
			if (slowStreamIntervalId) {
				clearInterval(slowStreamIntervalId)
				slowStreamIntervalId = undefined
			}
		}
	})
	$effect(() => {
		;[autoRefresh, refreshRate]
		untrack(() => {
			if (!intervalId && autoRefresh) {
				intervalId = setInterval(syncer, refreshRate)
			}
		})
	})
	$effect(() => {
		autoRefresh
		untrack(() => {
			if (intervalId && !autoRefresh) {
				clearInterval(intervalId)
				intervalId = undefined
			}
		})
	})

	return {
		loadExtraJobs,
		loadJobs,
		stopBatchLoading,
		restreamWithBatchSize,
		get batchProgress() {
			return batchProgress
		},
		get currentBatchSize() {
			return currentBatchSize
		},
		get scanProgress() {
			return scanProgress
		},
		get queue_count() {
			return queue_count
		},
		get suspended_count() {
			return suspended_count
		},
		get loading() {
			return loading
		},
		get loadingExtra() {
			return loadingExtra
		},
		get completedJobs() {
			return completedJobs
		},
		get externalJobs() {
			return externalJobs
		},
		get extendedJobs() {
			return extendedJobs
		},
		get jobs() {
			return jobs
		},
		get lastFetchWentToEnd() {
			return lastFetchWentToEnd
		}
	}
}
