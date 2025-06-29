<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext, untrack } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext, ContextPanelContext } from '../../types'
	import { recursivelyFilterKeyInJSON as recursivelyFilterInJSON } from '../appUtils'

	interface Props {
		componentId: string
		hasContent?: boolean
		suffix?: string
		render?: boolean
	}

	let { componentId, hasContent = $bindable(false), suffix = '', render = true }: Props = $props()
	const { worldStore, connectingInput } = getContext<AppViewerContext>('AppViewerContext')
	const { search, hasResult } = getContext<ContextPanelContext>('ContextPanel')

	let object = $state({})

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				object[k] = undefined
				output?.subscribe(
					{
						id: 'alloutputs-' + suffix + componentId + '-' + k,
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

	$effect(() => {
		$worldStore?.outputsById?.[componentId]
		untrack(() => {
			subscribeToAllOutputs($worldStore?.outputsById?.[componentId])
		})
	})
	let filtered = $derived(recursivelyFilterInJSON(object, $search, componentId))
	$effect(() => {
		$hasResult[componentId] = Object.keys(filtered).length > 0
	})
</script>

{#if render && object != undefined && Object.keys(object).length > 0}
	{#if $hasResult[componentId] || $search == ''}
		<div class="pl-2 !cursor-pointer" data-connection-button>
			<ObjectViewer
				json={filtered}
				on:select
				topBrackets={false}
				pureViewer={!$connectingInput.opened}
				allowCopy={!$connectingInput.opened}
				connecting={$connectingInput.opened}
			/>
		</div>
	{:else if $search.length > 0}
		<div class="text-xs pl-2 text-tertiary">No results</div>
	{:else}
		<div class="text-xs pl-2 text-tertiary">No outputs</div>
	{/if}
{/if}
