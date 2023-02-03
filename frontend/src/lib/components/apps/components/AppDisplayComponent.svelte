<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppInput } from '../inputType'
	import { IS_APP_PUBLIC_CONTEXT_KEY } from '../types'
	import RunnableWrapper from './helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined

	const requireHtmlApproval = getContext<boolean | undefined>(IS_APP_PUBLIC_CONTEXT_KEY)
	let result: any = undefined

	export const staticOutputs: string[] = ['result', 'loading']
</script>

<RunnableWrapper flexWrap bind:componentInput {id} bind:initializing bind:result>
	<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
		Results
	</div>
	<div class="p-2">
		<DisplayResult {result} {requireHtmlApproval} />
	</div>
</RunnableWrapper>
