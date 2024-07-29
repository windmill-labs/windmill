<script lang="ts" context="module">
	let loading: Writable<boolean> = writable(false)
</script>

<script lang="ts">
	import { Loader2, RefreshCw } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import Button from '../../common/button/Button.svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { allItems } from '../utils'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames } from '$lib/utils'
	import { twMerge } from 'tailwind-merge'
	import { writable, type Writable } from 'svelte/store'

	const { runnableComponents, app, initialized } = getContext<AppViewerContext>('AppViewerContext')
	const appEditorContext = getContext<AppEditorContext>('AppEditorContext')

	let timeout: NodeJS.Timeout | undefined = undefined
	let interval: number | undefined = undefined
	let shouldRefresh = false
	let firstLoad = false

	$: !firstLoad &&
		$initialized.initializedComponents?.length ==
			allItems($app.grid, $app.subgrids).length + ($app.hiddenInlineScripts?.length ?? 0) &&
		refresh()
	$: componentNumber = Object.values($runnableComponents).filter((x) => x.autoRefresh).length

	onMount(() => {
		if (appEditorContext) {
			appEditorContext.refreshComponents.set(refresh)
		}
		document.addEventListener('visibilitychange', visChange)
		// setTimeout(() => refresh(), 1000)
		return () => {
			document.removeEventListener('visibilitychange', visChange)
			if (timeout) clearInterval(timeout)
		}
	})

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

	let refreshing: string[] = []
	function refresh() {
		let isFirstLoad = false
		if (!firstLoad) {
			$initialized.initialized = true
			firstLoad = true
			isFirstLoad = true
		}
		$loading = true

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
					)
				}
			})
			.filter(Boolean)

		Promise.all(promises).finally(() => {
			$loading = false
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

	let items = [
		{
			displayName: 'Once',
			action: () => setInter(undefined)
		},
		...[1, 2, 3, 4, 5, 6].map((i) => ({
			displayName: `Every ${i * 5} seconds`,
			action: () => setInter(i * 5000)
		}))
	]
</script>

<!-- {$initialized.initializedComponents?.join(', ')}
{allItems($app.grid, $app.subgrids).length + $app.hiddenInlineScripts.length} -->
<!-- {$initialized.initializedComponents} -->
<!-- {allItems($app.grid, $app.subgrids)
	.map((x) => x.id)
	.filter((x) => !$initialized.initializedComponents?.includes(x))
	.sort()
	.join(', ')} -->
<!-- {allItems($app.grid, $app.subgrids).map((x) => x.id)} -->

<div class={twMerge('flex items-center border rounded-md')}>
	<Button
		disabled={componentNumber == 0}
		on:click={() => onClick()}
		color="light"
		size="xs"
		variant="border"
		btnClasses={twMerge(
			'!rounded-r-none text-tertiary !text-2xs !border-r border-y-0 border-l-0 group'
		)}
		title="Refresh {componentNumber} component{componentNumber > 1 ? 's' : ''} {interval
			? `every ${interval / 1000} seconds`
			: 'Once'} {refreshing.length > 0 ? `(live: ${refreshing.join(', ')}))` : ''}"
	>
		<div class="z-10 flex flex-row items-center gap-2">
			{#if !$loading}
				<RefreshCw size={14} />
			{:else}
				<Loader2 class="animate-spin text-blue-500" size={14} />
			{/if}

			&nbsp;Reload ({componentNumber})
		</div>
	</Button>

	<ButtonDropdown hasPadding={true} disabled={componentNumber == 0}>
		<svelte:fragment slot="label">
			<span class={twMerge('text-xs min-w-[2rem] ', interval ? 'text-blue-500' : 'text-tertiary')}>
				{interval ? `${interval / 1000}s` : 'Once'}
			</span>
		</svelte:fragment>
		<svelte:fragment slot="items">
			{#each items ?? [] as { }, index}
				<MenuItem
					on:click={() => {
						if (index === 0) {
							setInter(undefined)
						} else {
							setInter(index * 5000)
						}
					}}
				>
					<div
						class={classNames(
							'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-surface-hover !text-xs font-semibold'
						)}
					>
						{#if index === 0}
							Once
						{:else}
							{`Every ${index * 5} seconds`}
						{/if}
					</div>
				</MenuItem>
			{/each}
		</svelte:fragment>
	</ButtonDropdown>
</div>
