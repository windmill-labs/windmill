<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { allItems } from '../utils'
	import RecomputeAllButton from './RecomputeAllButton.svelte'

	const { runnableComponents, app, initialized, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')
	const appEditorContext = getContext<AppEditorContext>('AppEditorContext')

	let timeout: NodeJS.Timeout | undefined = undefined
	let interval: number | undefined = undefined
	let shouldRefresh = false
	let firstLoad = false
	let progressTimer: NodeJS.Timeout | undefined = undefined

	$: !firstLoad &&
		$initialized.initializedComponents?.length ==
			allItems($app.grid, $app.subgrids).length + ($app.hiddenInlineScripts?.length ?? 0) &&
		refresh()

	$: $recomputeAllContext.componentNumber =
		Object.values($runnableComponents).filter((x) => x.autoRefresh).length ?? 0

	onMount(() => {
		if (appEditorContext) {
			appEditorContext.refreshComponents.set(refresh)
		}
		document.addEventListener('visibilitychange', visChange)
		// setTimeout(() => refresh(), 1000)
		return () => {
			document.removeEventListener('visibilitychange', visChange)
			if (timeout) clearInterval(timeout)
			if (progressTimer) clearInterval(progressTimer)
		}
	})

	function onClick(stopAfterClear = false) {
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
		refresh()

		if (interval) {
			shouldRefresh = true
			timeout = setInterval(refresh, interval)
			startProgress()
		}
	}

	function startProgress() {
		$recomputeAllContext.progress = 100
		if (progressTimer) clearInterval(progressTimer)
		progressTimer = setInterval(() => {
			if ($recomputeAllContext.progress) {
				const newProgress = $recomputeAllContext.progress - 100 / ((interval ?? 1000) / 100)
				if (newProgress <= 0) {
					return 0
				}

				$recomputeAllContext.progress = newProgress
			}
		}, 100)
	}

	function setInter(inter: number | undefined) {
		interval = inter
		$recomputeAllContext.interval = inter
		onClick(!inter)
	}

	let refreshing: string[] = []
	function refresh() {
		let isFirstLoad = false
		if (!firstLoad) {
			$initialized.initialized = true
			firstLoad = true
			isFirstLoad = true
		}
		$recomputeAllContext.loading = true
		$recomputeAllContext.progress = 100

		console.log('refresh all')
		refreshing = []
		const promises = Object.keys($runnableComponents)
			.flatMap((id) => {
				if (
					!$runnableComponents?.[id]?.autoRefresh &&
					(!isFirstLoad || !$runnableComponents?.[id]?.refreshOnStart)
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
			timeout = setInterval(refresh, interval)
			startProgress()
		}
	}

	onMount(() => {
		$recomputeAllContext = {
			onClick,
			setInter
		}
	})
</script>

{$recomputeAllContext.componentNumber}
<RecomputeAllButton
	on:click={() => onClick()}
	{interval}
	{refreshing}
	componentNumber={$recomputeAllContext.componentNumber ?? 0}
	loading={$recomputeAllContext.loading}
	progress={$recomputeAllContext.progress}
	on:setInter={(e) => {
		setInter(e.detail)
		onClick(false)
	}}
/>
