<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { App, AppEditorContext, AppViewerContext } from '../types'
	import { allItems, BG_PREFIX } from '../utils'
	import RecomputeAllButton from './RecomputeAllButton.svelte'

	const { runnableComponents, app, initialized, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')
	const appEditorContext = getContext<AppEditorContext>('AppEditorContext')

	let timeout: NodeJS.Timeout | undefined = undefined
	let shouldRefresh = false
	let firstLoad = false
	let progressTimer: NodeJS.Timeout | undefined = undefined

	$: !firstLoad &&
		canInitializeAll($initialized?.initializedComponents, app.val) &&
		refresh('all initialized')

	// $: console.log('canInitializeAll', firstLoad, $initialized?.initializedComponents)
	function canInitializeAll(initialized: string[] | undefined, app: App) {
		// console.log(
		// 	'canInitializeAll',
		// 	JSON.stringify(
		// 		{
		// 			initialized: initialized,
		// 			items: allItems(app.grid, app.subgrids).map((x) => x.id),
		// 			// missing1: allItems(app.grid, app.subgrids).filter((x) => !initialized?.includes(x.id)),

		// 			missing2: initialized?.filter(
		// 				(x) =>
		// 					!allItems(app.grid, app.subgrids)
		// 						.map((x) => x.id)
		// 						.includes(x)
		// 			)
		// 		},
		// 		null,
		// 		2
		// 	),
		// 	initialized?.length ==
		// 		allItems(app.grid, app.subgrids).length + (app.hiddenInlineScripts?.length ?? 0)
		// )
		if (app.eagerRendering || app.lazyInitRequire == undefined) {
			return (
				initialized?.length ==
				allItems(app.grid, app.subgrids).length + (app.hiddenInlineScripts?.length ?? 0)
			)
		} else {
			return (
				app.hiddenInlineScripts?.every((x, i) => initialized?.includes(BG_PREFIX + i)) &&
				app.lazyInitRequire?.every((x) => initialized?.includes(x))
			)
		}
	}

	$: $recomputeAllContext.componentNumber =
		Object.values($runnableComponents).filter((x) => x.autoRefresh).length ?? 0

	onMount(() => {
		if (appEditorContext) {
			appEditorContext.refreshComponents.set(() => refresh('onClick global'))
		}
		document.addEventListener('visibilitychange', visChange)
		// setTimeout(() => refresh(), 1000)
		return () => {
			document.removeEventListener('visibilitychange', visChange)
			if (timeout) clearInterval(timeout)
			if (progressTimer) clearInterval(progressTimer)
		}
	})

	function onRefresh(stopAfterClear: boolean, source: string, excludeId?: string) {
		if (timeout) {
			clearInterval(timeout)
			timeout = undefined
			shouldRefresh = false
			if (progressTimer) {
				clearInterval(progressTimer)
				progressTimer = undefined
			}
			if (stopAfterClear) return
		}
		if (firstLoad) {
			refresh('onClick ' + source, excludeId)
		}

		if ($recomputeAllContext.interval) {
			shouldRefresh = true
			timeout = setInterval(() => refresh('onClick interval'), $recomputeAllContext.interval)
			startProgress()
		}
	}

	function startProgress() {
		$recomputeAllContext.progress = 100
		if (progressTimer) clearInterval(progressTimer)
		progressTimer = setInterval(() => {
			if ($recomputeAllContext.progress) {
				const newProgress =
					$recomputeAllContext.progress - 100 / (($recomputeAllContext.interval ?? 1000) / 100)
				if (newProgress <= 0) {
					return 0
				}

				$recomputeAllContext.progress = newProgress
			}
		}, 100)
	}

	function setInter(inter: number | undefined, source: string) {
		$recomputeAllContext.interval = inter
		onRefresh(!inter, 'setInter ' + source)
	}

	let refreshing: string[] = []
	function refresh(reason: string, excludeId: string | undefined = undefined) {
		let isFirstLoad = false
		if (!firstLoad && reason == 'all initialized') {
			console.log('refresh all first load', reason)
			$initialized.initialized = true
			firstLoad = true
			isFirstLoad = true
		}
		$recomputeAllContext.loading = true
		$recomputeAllContext.progress = 100

		console.log('refresh all', reason)
		refreshing = []
		const promises = Object.keys($runnableComponents)
			.flatMap((id) => {
				if (
					excludeId === id ||
					(!$runnableComponents?.[id]?.autoRefresh &&
						(!isFirstLoad || !$runnableComponents?.[id]?.refreshOnStart))
				) {
					return
				}

				let cb = $runnableComponents?.[id]?.cb
				if (cb) {
					console.log('refresh start', id)
					refreshing.push(id)
					return cb.map((f) =>
						f()
							.then(() => {
								console.log('refreshed', id)
								refreshing = refreshing.filter((x) => x !== id)
							})
							.catch((e) => {
								console.error('refresh error', id)
								refreshing = refreshing.filter((x) => x !== id)
							})
							.finally(() => {
								$recomputeAllContext.refreshing = refreshing
							})
					)
				}
			})
			.filter(Boolean)

		Promise.all(promises).finally(() => {
			$recomputeAllContext.loading = false
		})
	}

	function visChange() {
		if (document.visibilityState === 'hidden') {
			if (timeout) {
				clearInterval(timeout)
				timeout = undefined
				if (progressTimer) clearInterval(progressTimer)
			}
		} else if (shouldRefresh) {
			timeout = setInterval(() => refresh('onRefresh interval'), $recomputeAllContext.interval)
			startProgress()
		}
	}

	onMount(() => {
		$recomputeAllContext = {
			onRefresh: (excludeIds) => onRefresh(false, 'allContext', excludeIds),
			setInter: (n) => setInter(n, 'all context')
		}
	})
</script>

<RecomputeAllButton
	on:click={() => onRefresh(false, 'button')}
	interval={$recomputeAllContext.interval}
	{refreshing}
	componentNumber={$recomputeAllContext.componentNumber ?? 0}
	loading={$recomputeAllContext.loading}
	progress={$recomputeAllContext.progress}
	on:setInter={(e) => {
		setInter(e.detail, 'button setInter')
		onRefresh(false, 'button setInter')
	}}
/>
