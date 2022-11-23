<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, ComponentInputsSpec } from '../types'
	import InputValue from './helpers/InputValue.svelte'

	export let componentInputs: ComponentInputsSpec

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	let resultValue: any = undefined

	export const staticOutputs: string[] = []
</script>

<InputValue input={componentInputs.result} bind:value={resultValue} />

{#if $worldStore}
	<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
		Results
	</div>
	<div class="p-2">
		{#if resultValue === undefined && componentInputs.result.type === 'output'}
			<span class="text-sm">Waiting for result</span>
		{:else}
			<DisplayResult result={resultValue} />
		{/if}
	</div>
{/if}
