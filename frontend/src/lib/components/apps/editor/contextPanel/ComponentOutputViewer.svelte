<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { recursivelyFilterKeyInJSON as recursivelyFilterInJSON } from '../appUtils'

	export let componentId: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')
	const { search, hasResult } = getContext<ContextPanelContext>('ContextPanel')

	let object = {}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				object[k] = undefined
				output?.subscribe({
					id: 'alloutputs' + componentId + '-' + k,
					next: (value) => {
						object[k] = value
					}
				})
			})
		}
	}

	$: subscribeToAllOutputs($worldStore?.outputsById?.[componentId])

	$: filtered = recursivelyFilterInJSON(object, $search, componentId)

	$: $hasResult[componentId] = Object.keys(filtered).length > 0
</script>

{#if $hasResult[componentId] || $search == ''}
	<ObjectViewer json={filtered} on:select topBrackets={false} />
{:else if $search.length > 0}
	<div class="text-xs pl-2 text-gray-600">No results</div>
{:else}
	<div class="text-xs pl-2 text-gray-600">No outputs</div>
{/if}
