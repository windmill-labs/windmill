<script lang="ts">
	import { classNames } from '$lib/utils'
	import { grid } from 'd3-dag'
	import { getContext } from 'svelte'
	import type { ConnectedAppInput } from '../../inputType'
	import type { AppViewerContext } from '../../types'
	import { findGridItem } from '../appUtils'

	const { selectedComponent, app } = getContext<AppViewerContext>('AppViewerContext')

	$: gridItem = $selectedComponent ? findGridItem($app, $selectedComponent) : undefined

	let dependencies: Array<{ componentId: string; path: string }> | undefined = undefined

	$: if (gridItem && dependencies === undefined) {
		const fields =
			gridItem?.data?.componentInput?.type === 'runnable'
				? gridItem?.data?.componentInput?.fields
				: undefined

		if (!fields) {
			dependencies = []
		} else {
			dependencies = []

			Object.values(fields).forEach((field) => {
				if (field.type === 'connected' && dependencies && field.connection) {
					dependencies.push({
						componentId: field.connection?.componentId,
						path: field.connection?.path
					})
				}
			})
		}
	}

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		yellow: 'text-yellow-800 border-yellow-600 bg-yellow-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	<p class="text-xs font-semibold text-slate-800 ">Executions (2)</p>
	<div>
		{#if gridItem?.data.type === 'buttoncomponent'}
			<span class={classNames(badgeClass, colors['indigo'])}> On click </span>
		{:else}
			<span class={classNames(badgeClass, colors['green'])}> On load </span>
		{/if}
		{#each dependencies ?? [] as dependency}
			<span class={classNames(badgeClass, colors['yellow'])}>
				Trigger {dependency.componentId} : {dependency.path}
			</span>
		{/each}
	</div>
</div>
