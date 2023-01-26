<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import type { AppInput } from '../inputType'
	import RunnableWrapper from './helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined

	let result: any = undefined

	export const staticOutputs: string[] = ['result', 'loading']
</script>

<RunnableWrapper flexWrap bind:result bind:componentInput {id}>
	<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
		Results
	</div>
	<div class="p-2">
		<DisplayResult {result} requireHtmlApproval />
	</div>
</RunnableWrapper>
