import { untrack } from 'svelte'
import { AuditService, CancelError, CancelablePromise, type AuditLog } from '$lib/gen'
import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'
import { sendUserToast } from '$lib/toast'
import type { ActionKind } from '$lib/common'

export interface AuditLogsLoaderArgs {
	workspace: string | undefined
	scope: undefined | 'all_workspaces' | 'instance'
	username: string
	operation: string
	resource: string | undefined
	actionKind: ActionKind | 'all'
	before: string | undefined
	after: string | undefined
	perPage: number
}

const SMALL_BATCH_SIZE = 25
// The page size comes from the url, and a batched load turns it into one request per batch, so it
// has to be capped at the largest size the page itself offers.
const MAX_PER_PAGE = 1000

/**
 * Loads the newest audit logs, optionally streaming them in smaller batches so rows show up as
 * they arrive on instances where a full load takes a long time to come back, and appends older
 * ones on demand. Rows are ordered by descending id, so every batch after the first follows a
 * `before_id` cursor.
 */
export function useAuditLogsLoader(args: () => AuditLogsLoaderArgs) {
	let logs: AuditLog[] | undefined = $state()
	let loading = $state(false)
	let loadingExtra = $state(false)
	let hasMore = $state(false)
	let batchProgress = $state<{ loaded: number; total: number } | null>(null)
	let currentBatchSize = $state<number | null>(null)

	let pendingLoad: CancelablePromise<void> | undefined
	let pendingLoadHasRows = false
	let pendingExtra: CancelablePromise<void> | undefined

	function fetchBatch(
		a: AuditLogsLoaderArgs,
		limit: number,
		beforeId: number | undefined
	): CancelablePromise<AuditLog[]> {
		return AuditService.listAuditLogs({
			workspace: a.scope === 'instance' ? 'global' : a.workspace!,
			perPage: limit,
			beforeId,
			before: a.before,
			after: a.after,
			username: a.username === 'all' ? undefined : a.username,
			operation: a.operation === 'all' || a.operation === '' ? undefined : a.operation,
			resource: a.resource === 'all' || a.resource === '' ? undefined : a.resource,
			actionKind: a.actionKind === 'all' ? undefined : a.actionKind,
			allWorkspaces: a.scope === 'all_workspaces'
		})
	}

	function clearBatchState() {
		batchProgress = null
		currentBatchSize = null
	}

	/**
	 * A load that stops or fails never completed its page: it says nothing about whether a next
	 * page exists, and with no rows of its own the rows of the query it replaced would stand in
	 * for its result.
	 */
	function abandonLoad() {
		if (!pendingLoadHasRows) {
			logs = []
		}
		hasMore = false
		clearBatchState()
		loading = false
	}

	function load(batchSize?: number): CancelablePromise<void> {
		pendingLoad?.cancel()
		pendingLoad = undefined
		pendingLoadHasRows = false
		cancelExtra()

		const a = args()
		if (a.workspace == undefined && a.scope !== 'instance') {
			loading = false
			clearBatchState()
			return CancelablePromiseUtils.pure<void>(undefined)
		}
		const total = Math.min(Math.max(1, Math.floor(a.perPage) || 1), MAX_PER_PAGE)
		const size = Math.min(Math.max(1, batchSize ?? total), total)
		const isBatched = size < total

		loading = true
		batchProgress = isBatched ? { loaded: 0, total } : null
		currentBatchSize = isBatched ? size : null

		const acc: AuditLog[] = []
		let slowBatchToastShown = false

		function loadBatch(beforeId: number | undefined): CancelablePromise<void> {
			let fetch = fetchBatch(a, size, beforeId)
			if (isBatched && size > 1) {
				fetch = CancelablePromiseUtils.onTimeout(fetch, 4000, () => {
					if (slowBatchToastShown) return
					slowBatchToastShown = true
					sendUserToast(
						`Streaming by batches of ${size} is slow, try loading one at a time`,
						'warning',
						[{ label: 'Stream 1 by 1', callback: () => restreamWithBatchSize(1) }]
					)
				})
			}
			return CancelablePromiseUtils.then(fetch, (rows) => {
				acc.push(...rows.slice(0, total - acc.length))
				logs = [...acc]
				loading = false
				pendingLoadHasRows = true
				if (isBatched) {
					batchProgress = { loaded: acc.length, total }
				}
				if (rows.length < size || acc.length >= total) {
					// Only once the load is complete: a half-streamed one says nothing about
					// whether older logs exist.
					hasMore = acc.length >= total
					return CancelablePromiseUtils.pure<void>(undefined)
				}
				return loadBatch(rows[rows.length - 1].id)
			})
		}

		let slowLoadIntervalId: ReturnType<typeof setInterval> | undefined
		if (isBatched) {
			slowLoadIntervalId = setInterval(() => {
				sendUserToast(
					'Loading is taking a long time...',
					'warning',
					[{ label: 'Stop loading', callback: () => stopBatchLoading() }],
					undefined,
					8000
				)
			}, 15000)
		}

		let promise = loadBatch(undefined)
		if (!isBatched) {
			promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
				const smaller = total > SMALL_BATCH_SIZE ? SMALL_BATCH_SIZE : 1
				sendUserToast(
					'Loading audit logs is taking longer than expected...',
					'warning',
					total > 1
						? [
								{
									label: smaller === 1 ? 'Stream 1 by 1' : `Stream by batches of ${smaller}`,
									callback: () => restreamWithBatchSize(smaller)
								}
							]
						: []
				)
			})
		}
		promise = CancelablePromiseUtils.finallyDo(promise, () => {
			if (slowLoadIntervalId) clearInterval(slowLoadIntervalId)
		})
		// Only on success: a cancel means another load already owns these.
		promise = CancelablePromiseUtils.pipe(promise, clearBatchState)
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.pure<void>(undefined)
			abandonLoad()
			sendUserToast(
				'There was an issue loading audit logs, see browser console for more details',
				true
			)
			console.error(e)
			return CancelablePromiseUtils.pure<void>(undefined)
		})
		const thisLoad = promise
		// The "Stop loading" toast outlives the load it was raised for, so a settled load has to
		// stop being the pending one.
		CancelablePromiseUtils.pipe(thisLoad, () => {
			if (pendingLoad === thisLoad) pendingLoad = undefined
		})
		pendingLoad = thisLoad
		return thisLoad
	}

	function restreamWithBatchSize(batchSize: number) {
		load(batchSize)
	}

	function stopBatchLoading() {
		if (!pendingLoad) return
		pendingLoad.cancel()
		pendingLoad = undefined
		abandonLoad()
	}

	function cancelExtra() {
		pendingExtra?.cancel()
		pendingExtra = undefined
		loadingExtra = false
	}

	function loadMore() {
		const last = logs?.[logs.length - 1]
		if (!last || !hasMore || pendingLoad || pendingExtra) return
		const a = args()
		const total = Math.min(Math.max(1, Math.floor(a.perPage) || 1), MAX_PER_PAGE)
		loadingExtra = true
		let promise = CancelablePromiseUtils.then(fetchBatch(a, total, last.id), (rows) => {
			logs = [...(logs ?? []), ...rows]
			hasMore = rows.length >= total
			return CancelablePromiseUtils.pure<void>(undefined)
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.pure<void>(undefined)
			sendUserToast(
				'There was an issue loading more audit logs, see browser console for more details',
				true
			)
			console.error(e)
			return CancelablePromiseUtils.pure<void>(undefined)
		})
		const thisExtra = CancelablePromiseUtils.pipe(promise, () => {
			if (pendingExtra === thisExtra) {
				pendingExtra = undefined
				loadingExtra = false
			}
		})
		pendingExtra = thisExtra
	}

	$effect(() => {
		// Building the args reads every filter, which is what registers this effect's dependencies.
		args()
		untrack(() => load())
		return () => {
			pendingLoad?.cancel()
			pendingLoad = undefined
			cancelExtra()
		}
	})

	return {
		reload: () => load(),
		restreamWithBatchSize,
		stopBatchLoading,
		loadMore,
		get logs() {
			return logs
		},
		get loading() {
			return loading
		},
		get loadingExtra() {
			return loadingExtra
		},
		get hasMore() {
			return hasMore
		},
		get batchProgress() {
			return batchProgress
		},
		get currentBatchSize() {
			return currentBatchSize
		}
	}
}
