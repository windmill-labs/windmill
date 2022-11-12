<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, ComponentInputsSpec } from '../types'

	export let componentInputs: ComponentInputsSpec

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: inputResult =
		componentInputs.result.id && componentInputs.result.name
			? $worldStore?.connect<any>(componentInputs.result, (x) => {
					update()
			  })
			: undefined

	let result: any

	function update() {
		result = inputResult?.peak()
	}

	export const staticOutputs: string[] = []
</script>

{#if $worldStore}
	<DisplayResult {result} />
{/if}
