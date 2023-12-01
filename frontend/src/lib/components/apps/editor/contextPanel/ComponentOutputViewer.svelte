<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { recursivelyFilterKeyInJSON as recursivelyFilterInJSON } from '../appUtils'

	export let componentId: string
	export let hasContent: boolean = false
	export let suffix: string = ''

	const { worldStore, connectingInput } = getContext<AppViewerContext>('AppViewerContext')
	const { search, hasResult } = getContext<ContextPanelContext>('ContextPanel')

	let object = {}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				object[k] = undefined
				output?.subscribe(
					{
						id: 'alloutputs' + suffix + componentId + '-' + k,
						next: (value) => {
							if (!hasContent) {
								hasContent = true
							}
							object[k] = value
						}
					},
					object[k]
				)
			})
		}
	}

	$: subscribeToAllOutputs($worldStore?.outputsById?.[componentId])
	$: filtered = recursivelyFilterInJSON(object, $search, componentId)
	$: $hasResult[componentId] = Object.keys(filtered).length > 0
</script>

{#if object != undefined && Object.keys(object).length > 0}
	{#if $hasResult[componentId] || $search == ''}
		<ObjectViewer
			json={filtered}
			on:select
			topBrackets={false}
			pureViewer={!$connectingInput.opened}
		/>
	{:else if $search.length > 0}
		<div class="text-xs pl-2 text-tertiary">No results</div>
	{:else}
		<div class="text-xs pl-2 text-tertiary">No outputs</div>
	{/if}
{/if}
