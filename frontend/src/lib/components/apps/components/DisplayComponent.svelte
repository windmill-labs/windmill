<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import type { World } from '../rx'
	import type { AppInputTransform } from '../types'

	export let world: World | undefined
	export let inputs: {
		result: AppInputTransform
	}

	$: inputResult = world?.connect(inputs.result, (x) => {
		debugger
		update()
	})

	let res: any
	function update() {
		debugger
		res = inputResult?.peak()
	}

	export function getOutputs() {
		return []
	}
</script>

{res}
{#if world}
	<DisplayResult result={res} />
{/if}
