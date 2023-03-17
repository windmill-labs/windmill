<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { classNames } from '$lib/utils'
	import { getContext } from 'svelte'
	import { getAllRecomputeIdsForComponent } from '../../appUtils'
	import type { DependencyBadge } from './triggerListUtils'

	export let valuesChangeBadges: DependencyBadge = []
	export let onClick: boolean = false
	export let onLoad: boolean = false
	export let id: string | undefined = undefined

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		yellow: 'text-yellow-800 border-yellow-600 bg-yellow-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		orange: 'text-orange-800 border-orange-600 bg-orange-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'

	const recomputedBadges: DependencyBadge = []
	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	$: if ($app && $selectedComponent && id) {
		const recomputeIds = getAllRecomputeIdsForComponent($app, id)

		if ($selectedComponent && recomputeIds) {
			recomputeIds.forEach((x) => {
				recomputedBadges.push({
					label: `Recomputed by: ${x}`,
					color: 'indigo'
				})
			})
		}
	}
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	{#if recomputedBadges.length === 0 && !onLoad && !onClick && valuesChangeBadges.length === 0}
		<p class="text-xs font-semibold text-slate-800 ">
			This script has no triggers. It will never run.
		</p>
	{:else}
		<div class="text-sm font-semibold text-slate-800 ">Triggered by</div>

		<div>
			<div class="text-xs font-semibold text-slate-800 mb-1">Event</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#if onLoad}
					<span class={classNames(badgeClass, colors['green'])}>On load</span>
				{/if}
				{#if onClick}
					<span class={classNames(badgeClass, colors['blue'])}>On click</span>
				{/if}
			</div>
		</div>
		{#if valuesChangeBadges.length > 0}
			<div>
				<div class="text-xs font-semibold text-slate-800 mb-1">Values change</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each valuesChangeBadges as badge}
						<span class={classNames(badgeClass, colors[badge.color])}>{badge.label}</span>
					{/each}
				</div>
			</div>
		{/if}
		{#if recomputedBadges.length > 0}
			<div>
				<div class="text-xs font-semibold text-slate-800 mb-1">Success of</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each recomputedBadges as badge}
						<span class={classNames(badgeClass, colors[badge.color])}>{badge.label}</span>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
