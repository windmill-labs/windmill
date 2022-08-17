<script lang="ts">
	import {
		faClose,
		faFlagCheckered,
		faInfoCircle,
		faPen,
		faPlus,
		faSliders,
		faTrashAlt
	} from '@fortawesome/free-solid-svg-icons'
	import { Button, Toggle, Tooltip } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import { Highlight } from 'svelte-highlight'
	import typescript from 'svelte-highlight/languages/typescript'
	import ModuleStep from '../ModuleStep.svelte'
	import ObjectViewer from '../propertyPicker/ObjectViewer.svelte'
	import FlowInput from './FlowInput.svelte'
	import type { FlowState } from './flowState'
	import { emptyFlowModuleSchema } from './flowStateUtils'
	import { stepOpened } from './stepOpenedStore'

	export let args: Record<string, any> = {}
	export let flowModuleSchemas: FlowState
	export let parentIndex: number | undefined = undefined

	const root = parentIndex === undefined

	function insertAtIndex(index: number): void {
		flowModuleSchemas.splice(index, 0, emptyFlowModuleSchema())
		flowModuleSchemas = flowModuleSchemas

		const indexes = getIndexes(parentIndex, index)
		stepOpened.update(() => String(indexes.join('-')))
	}

	function removeAtIndex(index: number): void {
		flowModuleSchemas.splice(index, 1)
		flowModuleSchemas = flowModuleSchemas
	}

	function getIndexes(parentIndex: number | undefined, childIndex: number): number[] {
		const indexes: number[] = []

		if (parentIndex !== undefined) {
			indexes.push(parentIndex)
		}
		indexes.push(childIndex)

		return indexes
	}

	const color = root ? 'blue' : 'orange'
</script>

<div class="w-full">
	<ol class="relative ml-4 border-l border-gray-200 dark:border-gray-700 space-y-12 border-dashed">
		{#if root}
			<li class="ml-8">
				<span class="relative">
					<span
						class={`flex absolute -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
					>
						<Icon
							class={`text-${color}-600 dark:text-${color}-400  text-font-bold text-center`}
							data={faSliders}
						/>
					</span>
				</span>
				<slot name="settings" />
			</li>
			<li class="ml-8 ">
				<span class="relative">
					<span
						class={`flex absolute top-4 -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
					>
						<Icon
							class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}
							data={faPen}
						/>
					</span>
				</span>
				<FlowInput />
			</li>
		{/if}
		{#each flowModuleSchemas as flowModuleSchema, index (index)}
			{#if flowModuleSchema.flowModule.value.type === 'forloopflow'}
				<li id="module-{index}" class="ml-4 relative">
					<button
						on:click={() => insertAtIndex(index)}
						class="flex absolute -top-10 -left-8 justify-center items-center bg-white border-2 border-gray-200 w-8 h-8 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900"
					>
						<Icon class="text-gray-400" data={faPlus} />
					</button>
					<div
						class="py-2 px-6 ml-4 mb-16 flex justify-between text-sm font-bold border border-gray-300 rounded-md shadow-md"
						role="alert"
					>
						<span class="flex items-center z-50">
							For loop
							<Tooltip
								content="Inside a loop, the flow input has an 'iter' property. It contains the value and the index of the iteration"
								placement="bottom"
								arrow
							>
								<Icon data={faInfoCircle} class="ml-2" /></Tooltip
							>
						</span>
						<span class="flex items-center space-x-2">
							<Toggle size="small" bind:checked={flowModuleSchema.flowModule.skip_if_stopped}>
								Skip if stopped
							</Toggle>

							<Button size="xs" color="alternative" on:click={() => removeAtIndex(index)}>
								<Icon data={faTrashAlt} class="mr-2" />
								Remove loop
							</Button>
						</span>
					</div>
					<span
						class="flex absolute top-3 -left-8 justify-center items-center w-8 h-8 bg-orange-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-orange-900"
					>
						<span class={`text-orange-600 dark:text-orange-400 font-bold text-center`}>
							{index + 1}
						</span>
					</span>

					<svelte:self
						bind:args
						bind:flowModuleSchemas={flowModuleSchema.childFlowModules}
						parentIndex={index}
					/>
					<span
						class="flex absolute bottom-3 -left-8 justify-center items-center w-8 h-8 bg-orange-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-orange-900"
					>
						<Icon
							class="text-orange-600 dark:text-orange-400 font-bold text-center"
							data={faFlagCheckered}
						/>
					</span>

					<div
						class="flex px-6 py-4 ml-4 mt-4 text-sm font-bold border border-gray-300 rounded-md shadow-md z-50"
						role="alert"
					>
						End of For Loop
						<Tooltip
							content="The results of each iteration are collecting in an array and is the result of the step.	"
							placement="bottom"
							arrow
						>
							<Icon data={faInfoCircle} class="ml-2" /></Tooltip
						>
					</div>
				</li>
			{:else}
				<li id="module-{String(getIndexes(parentIndex, index).join('-'))}" class="ml-8">
					<span class="relative">
						<button
							on:click={() => insertAtIndex(index)}
							class="flex absolute -top-10 -left-12 justify-center items-center bg-white border-2 border-gray-200 w-8 h-8 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900"
						>
							<Icon class="text-gray-400" data={faPlus} />
						</button>
						<span
							class={`flex absolute top-4 -left-12 justify-center items-center w-8 h-8 bg-${color}-200  rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
						>
							<span class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}>
								{index + 1}
							</span>
						</span>
						<ModuleStep
							indexes={getIndexes(parentIndex, index)}
							bind:mod={flowModuleSchema.flowModule}
							bind:args
							bind:schema={flowModuleSchema.schema}
							bind:childFlowModules={flowModuleSchema.childFlowModules}
							bind:previewResults={flowModuleSchema.previewResults}
							on:delete={() => removeAtIndex(index)}
							previousStepPreviewResults={index === 0
								? []
								: flowModuleSchemas[index - 1].previewResults}
						/>
						{#if flowModuleSchemas.length - 1 === index}
							<span
								class={`flex absolute bottom-0 -left-12 justify-center items-center w-8 h-8 bg-${color}-200 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900`}
							>
								<Icon
									class={`text-${color}-600 dark:text-${color}-400 font-bold text-center`}
									data={faFlagCheckered}
								/>
							</span>
						{/if}
					</span>
				</li>
			{/if}
			{#if flowModuleSchemas.length - 1 === index}
				<button
					on:click={() => insertAtIndex(flowModuleSchemas.length)}
					class="-ml-4 -mt-4 flex justify-center items-center bg-white border-2 border-gray-200 w-8 h-8 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900"
				>
					<Icon class="text-gray-400" data={faPlus} />
				</button>
			{/if}
		{/each}
		{#if flowModuleSchemas.length === 0}
			<button
				on:click={() => insertAtIndex(0)}
				class="-ml-4 flex justify-center items-center bg-white border-2 border-gray-200 w-8 h-8 rounded-full ring-8 ring-white dark:ring-gray-900 dark:bg-${color}-900"
			>
				<Icon class="text-gray-400" data={faPlus} />
			</button>
		{/if}
	</ol>
</div>
