<script lang="ts">
	import type { StaticInput } from '$lib/components/apps/inputType'
	import type { Output } from '$lib/components/apps/rx'
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { getContext } from 'svelte'

	export let componentInput: StaticInput<{ id: string; step: number }>

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const { worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const stepperComponents = allItems($app.grid, $app.subgrids).filter(
		(component) => component.data.type === 'steppercomponent'
	)

	let numberOfSteps = 0

	function subscribeToResultOutput(observableOutputs: Record<string, Output<any>> | undefined) {
		if (observableOutputs) {
			Object.entries(observableOutputs).forEach(([k, output]) => {
				if (k === 'result') {
					output?.subscribe({
						id: 'result',
						next: (value) => {
							numberOfSteps = Array.isArray(value) ? value.length : 0
						}
					})
				}
			})
		}
	}

	$: componentInput?.value?.id &&
		subscribeToResultOutput($worldStore?.outputsById?.[componentInput.value.id])
</script>

{#if componentInput.value && stepperComponents.length > 0}
	<div>
		<div class="flex flex-row gap-2 w-full">
			<div class="flex flex-col">
				<label for="stepId" class="text-xs font-semibold">Step ID</label>

				<select
					id="stepId"
					bind:value={componentInput.value.id}
					class="border border-gray-300 rounded-md p-1 !w-16"
				>
					{#each stepperComponents as stepComponent}
						<option value={stepComponent.data.id}>
							{stepComponent.data.id}
						</option>
					{/each}
				</select>
			</div>
			<div class="flex flex-col">
				<label for="stepIndex" class="text-xs font-semibold">Step Index</label>
				<select
					id="stepIndex"
					bind:value={componentInput.value.step}
					class="border border-gray-300 rounded-md p-1 !w-16"
				>
					{#each stepperComponents as stepComponent}
						{#if stepComponent.data.id === componentInput.value.id}
							{#each Array(numberOfSteps + 1) as _, i}
								<option value={i}>{i}</option>
							{/each}
						{/if}
					{/each}
				</select>
			</div>
		</div>
	</div>
{:else}
	<div class="text-xs text-gray-500"> No stepper component found in the app </div>
{/if}
