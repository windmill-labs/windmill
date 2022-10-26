<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import type { World } from '../rx'
	import type { AppInputTransform } from '../types'

	export let world: World | undefined
	export let inputs: {
		result: AppInputTransform
	}

	$: inputResult = world?.connect<any>(inputs.result, (x) => {
		update()
	})

	let result: any

	function update() {
		result = inputResult?.peak()
	}

	export const staticOutputs: string[] = []
</script>

{#if world}
	<DisplayResult {result} />
{/if}
