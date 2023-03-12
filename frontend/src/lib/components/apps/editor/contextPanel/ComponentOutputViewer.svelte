<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppViewerContext } from '../../types'

	export let outputs: string[] = []
	export let componentId: string

	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let object = {}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>>) {
		if (observableOutputs) {
			outputs.forEach((output: string) => {
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
</script>

{#if Object.keys(object).length > 0}
	<ObjectViewer json={object} on:select topBrackets={false} />
{/if}
