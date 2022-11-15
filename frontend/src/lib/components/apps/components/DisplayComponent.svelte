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
	<div class="border flex flex-col h-full shadow-sm rounded-sm">
		<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
			Results
		</div>
		<div class="p-2">
			<DisplayResult {result} />
		</div>
	</div>
{/if}
