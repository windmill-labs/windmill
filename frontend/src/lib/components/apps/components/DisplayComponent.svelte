<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import type { ComponentInput } from '../inputType'
	import RunnableWrapper from './helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: ComponentInput | undefined
	export const staticOutputs: string[] = []

	let result: any = undefined
</script>

<RunnableWrapper bind:result {id} {componentInput}>
	<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
		Results
	</div>
	<div class="p-2">
		<DisplayResult {result} />
	</div>
</RunnableWrapper>
