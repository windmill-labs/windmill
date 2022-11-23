<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { Output } from '../../rx'
	import type { AppEditorContext } from '../../types'

	export let outputs: string[] = []
	export let componentId: string

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let object = {}

	function subscribeToAllOutputs(observableOutputs: Record<string, Output<any>>) {
		if (observableOutputs) {
			outputs.forEach((output) => {
				observableOutputs[output].subscribe({
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

<ObjectViewer json={object} on:select topBrackets={true} />
