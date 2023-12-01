<script lang="ts">
	import { RefreshCw } from 'lucide-svelte'
	import { getContext, onMount } from 'svelte'
	import Button from '../../common/button/Button.svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { allItems } from '../utils'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'
	import { classNames } from '$lib/utils'

	const { runnableComponents, app, initialized } = getContext<AppViewerContext>('AppViewerContext')
	const appEditorContext = getContext<AppEditorContext>('AppEditorContext')

	let loading: boolean = false
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

	function refresh() {
		let isFirstLoad = false
		if (!firstLoad) {
			$initialized.initialized = true
			firstLoad = true
			isFirstLoad = true
		}
		loading = true

		const promises = Object.keys($runnableComponents)
			.flatMap((id) => {
				if (
					!$runnableComponents?.[id]?.autoRefresh &&
					(!isFirstLoad || !$runnableComponents?.[id]?.refreshOnStart)
				) {
					return
				}

				return $runnableComponents?.[id]?.cb?.map((f) =>
					f().then(() => console.log('refreshed', id))
				)
			})
			.filter(Boolean)

		Promise.all(promises).finally(() => {
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

<!-- {$initialized.initializedComponents?.join(', ')} -->
<!-- {allItems($app.grid, $app.subgrids).length + $app.hiddenInlineScripts.length}
{$initialized.initializedComponents}
{allItems($app.grid, $app.subgrids)
	.map((x) => x.id)
	.filter((x) => !$initialized.initializedComponents?.includes(x))
	.sort()
	.join(', ')} -->
<!-- {allItems($app.grid, $app.subgrids).map((x) => x.id)} -->

<div class="flex items-center">
	<Button
		disabled={componentNumber == 0}
		on:click={() => onClick()}
		color={timeout ? 'blue' : 'light'}
		variant={timeout ? 'contained' : 'border'}
		size="xs"
		btnClasses="!rounded-r-none text-tertiary !text-2xs {timeout ? '!border !border-blue-500' : ''}"
		title="Refresh {componentNumber} component{componentNumber > 1 ? 's' : ''} {interval
			? `every ${interval / 1000} seconds`
			: 'once'}"
	>
		<RefreshCw class={loading ? 'animate-spin' : ''} size={14} /> &nbsp;{componentNumber}
	</Button>

	<ButtonDropdown hasPadding={true}>
		<svelte:fragment slot="label">
			<span
				class={classNames('text-xs min-w-[2rem]', interval ? 'text-blue-500' : 'text-tertiary')}
			>
				{interval ? `${interval / 1000}s` : 'once'}
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
							'!text-tertiary text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
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
