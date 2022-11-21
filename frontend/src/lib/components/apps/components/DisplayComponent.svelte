<script lang="ts">
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, ComponentInputsSpec } from '../types'

	export let componentInputs: ComponentInputsSpec

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: hasConnection =
		componentInputs.result.type === 'output' &&
		componentInputs.result.id &&
		componentInputs.result.name

	$: inputResult = hasConnection
		? $worldStore?.connect<any>(componentInputs.result, () => {
				update()
		  })
		: {
				peak: () => {
					if (componentInputs.result.type === 'static') {
						return componentInputs.result.value
					}
				}
		  }

	let result: any

	function update() {
		result = inputResult?.peak()
	}

	$: !hasConnection && componentInputs.result && update()

	export const staticOutputs: string[] = []
</script>

{#if $worldStore}
	<div class="w-full border-b px-2 text-xs p-1 font-semibold bg-gray-500 text-white rounded-t-sm">
		Results
	</div>
	<div class="p-2">
		{#if !hasConnection && componentInputs.result.type !== 'static'}
			<span class="text-sm">Not connected</span>
		{:else if result === undefined && componentInputs.result.type === 'output'}
			<span class="text-sm">Waiting for result</span>
		{:else}
			<DisplayResult {result} />
		{/if}
	</div>
{/if}
