<script lang="ts">
	import Dropdown from '$lib/components/Dropdown.svelte'
	import { ChevronDown, RefreshCw } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import Button from '../../common/button/Button.svelte'
	import type { AppViewerContext } from '../types'
	import { allItems } from '../utils'

	const { runnableComponents, app, worldStore } = getContext<AppViewerContext>('AppViewerContext')
	let loading: boolean = false
	let timeout: NodeJS.Timer | undefined = undefined
	let interval: number | undefined = undefined
	let shouldRefresh = false
	let firstLoad = false

	$: !firstLoad &&
		$worldStore.initializedOutputs ==
			allItems($app.grid, $app.subgrids).length + $app.hiddenInlineScripts.length &&
		refresh()
	$: componentNumber = Object.values($runnableComponents).filter((x) => x.autoRefresh).length

	function onClick(stopAfterClear = true) {
		if (timeout) {
			clearInterval(timeout)
			timeout = undefined
			shouldRefresh = false
			if (stopAfterClear) return
		}
		refresh()
		if (interval) {
			shouldRefresh = true
			timeout = setInterval(refresh, interval)
		}
	}

	function setInter(inter: number | undefined) {
		interval = inter
		onClick(!inter)
	}

	function refresh() {
		if (!firstLoad) {
			$worldStore.initialized = true
			firstLoad = true
		}
		loading = true
		Promise.all(
			Object.keys($runnableComponents).map((id) => {
				if (!$runnableComponents?.[id]?.autoRefresh) {
					return
				}
				return $runnableComponents?.[id]?.cb?.()
			})
		).finally(() => {
			loading = false
		})
	}

	function visChange() {
		if (document.visibilityState === 'hidden') {
			if (timeout) {
				clearInterval(timeout)
				timeout = undefined
			}
		} else if (shouldRefresh) {
			timeout = setInterval(refresh, interval)
		}
	}

	onMount(() => {
		document.addEventListener('visibilitychange', visChange)
		// setTimeout(() => refresh(), 1000)
		return () => {
			document.removeEventListener('visibilitychange', visChange)
			if (timeout) clearInterval(timeout)
		}
	})
</script>

<div class="flex items-center">
	<Button
		disabled={componentNumber == 0}
		on:click={() => onClick()}
		color={timeout ? 'blue' : 'light'}
		variant={timeout ? 'contained' : 'border'}
		size="xs"
		btnClasses="!rounded-r-none {timeout ? '!border !border-blue-500' : ''}"
		title="Refresh {componentNumber} component{componentNumber > 1 ? 's' : ''} {interval
			? `every ${interval / 1000} seconds`
			: 'once'}"
	>
		<RefreshCw class={loading ? 'animate-spin' : ''} size={16} /> &nbsp;({componentNumber})
	</Button>
	<Dropdown
		btnClasses="!rounded-l-none !border-l-0 min-w-[4rem] !px-2"
		color={timeout ? 'blue' : 'light'}
		variant="border"
		dropdownItems={[
			{
				displayName: 'Once',
				action: () => setInter(undefined)
			},
			...[1, 2, 3, 4, 5, 6].map((i) => ({
				displayName: `Every ${i * 5} seconds`,
				action: () => setInter(i * 5000)
			}))
		]}
	>
		<span class="grow text center">{interval ? `${interval / 1000}s` : 'once'}</span>
		<ChevronDown class="ml-0.5" size={14} />
	</Dropdown>
</div>
