<script lang="ts">
	import ObjectViewer from '$lib/components/propertyPicker/ObjectViewer.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext } from '../../types'

	export let outputs: string[] = []
	export let componentId: string

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let object = {}

	outputs.forEach((output) => {
		console.log({ output })
		$worldStore?.outputsById[componentId][output].subscribe({
			next: (value) => {
				object[output] = value
			}
		})
	})
</script>

<ObjectViewer pureViewer={true} json={object} />
