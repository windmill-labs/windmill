<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppInputTransform } from '../types'

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')
	export let inputs: {
		result: AppInputTransform
	}

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
