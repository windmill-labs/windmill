<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, InputsSpec } from '../types'

	export let inputs: InputsSpec

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: inputResult = $worldStore?.connect<any>(inputs.result, (x) => {
		update()
	})

	let result: any

	function update() {
		result = inputResult?.peak()
	}

	export const staticOutputs: string[] = []
</script>

{#if $worldStore}
	<DisplayResult {result} />
{/if}
