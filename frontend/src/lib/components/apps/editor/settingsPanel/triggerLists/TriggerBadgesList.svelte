<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { classNames } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { getAllRecomputeIdsForComponent } from '../../appUtils'

	export let inputDependencies: string[] = []
	export let frontendDependencies: string[] | undefined = undefined
	export let onClick: boolean = false
	export let onLoad: boolean = false
	export let id: string | undefined = undefined

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'

	const recomputedBadges: string[] = []
	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')
	const dispatch = createEventDispatcher()

	$: if ($app && $selectedComponent && id) {
		const recomputeIds = getAllRecomputeIdsForComponent($app, id)

		if ($selectedComponent && recomputeIds) {
			recomputeIds.forEach((x) => {
				recomputedBadges.push(x)
			})
		}
	}
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	{#if recomputedBadges.length === 0 && !onLoad && !onClick && inputDependencies?.length === 0 && !frontendDependencies}
		<p class="text-xs font-semibold text-slate-800 ">
			This script has no triggers. It will never run.
		</p>
	{:else}
		<div class="text-sm font-semibold text-slate-800 ">Triggered by</div>

		{#if onLoad || onClick}
			<div class="w-full">
				<div class="text-xs font-semibold text-slate-800 mb-1">Events</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#if onLoad}
						<span class={classNames(badgeClass, colors['green'])}>Start</span>
						<span class={classNames(badgeClass, colors['green'])}>Refresh</span>
					{/if}
					{#if onClick}
						<span class={classNames(badgeClass, colors['green'])}>Click</span>
					{/if}
				</div>
			</div>
		{/if}
		{#if inputDependencies.length > 0}
			<div class="w-full">
				<div class="flex justify-between items-center mb-2">
					<div class="text-xs font-semibold text-slate-800 mb-1">Values change </div>
				</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each inputDependencies as label, index}
						<span class={classNames(badgeClass, colors['blue'])}>
							{label}
						</span>
					{/each}
				</div>
			</div>
		{/if}

		{#if recomputedBadges?.length > 0}
			<div class="w-full">
				<div class="text-xs font-semibold text-slate-800 mb-1">Computation of</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each recomputedBadges as badge}
						<span class={classNames(badgeClass, colors['indigo'])}>{badge}</span>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
	{#if frontendDependencies}
		<div class="w-full">
			<div class="flex justify-between items-center mb-2">
				<div class="text-xs font-semibold text-slate-800 mb-1">Values change (Frontend)</div>
				<slot />
			</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each frontendDependencies as label, index}
					<span class={classNames(badgeClass, colors['red'])}>
						{label}
						<button
							on:click={() => dispatch('delete', { index })}
							class="bg-red-300 cursor-pointer hover:bg-red-400 ml-1 rounded-md"
						>
							<X size={18} class="p-0.5" />
						</button>
					</span>
				{/each}
			</div>
		</div>
	{/if}
</div>
