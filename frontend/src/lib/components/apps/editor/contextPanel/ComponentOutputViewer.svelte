<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { Writable } from 'svelte/store'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'
	import { recursivelyFilterKeyInJSON } from '../appUtils'

	export let outputs: string[] = []
	export let componentId: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')
	const { search } = getContext<{ search: Writable<string> }>('searchCtx')

	let object = {}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>>) {
		if (observableOutputs) {
			outputs?.forEach((output: string) => {
				object[output] = undefined
				observableOutputs[output]?.subscribe({
					next: (value) => {
						object[output] = value
					}
				})
			})
		}
	}

	$: $worldStore?.outputsById[componentId] &&
		subscribeToAllOutputs($worldStore.outputsById[componentId])

	$: filtered = recursivelyFilterKeyInJSON(object, $search, componentId)
</script>

{#if Object.keys(filtered).length > 0}
	<ObjectViewer json={filtered} on:select topBrackets={false} />
{:else if $search.length > 0}
	<div class="text-xs pl-2">No results</div>
{/if}
