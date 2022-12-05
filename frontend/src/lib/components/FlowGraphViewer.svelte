<script lang="ts">
	import { FlowGraph } from './graph'
	import HighlightCode from './HighlightCode.svelte'
	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'
	import { scriptPathToHref } from '../utils'
	import type { FlowModule, FlowValue } from '$lib/gen'
	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let overflowAuto = false
	let stepDetail: FlowModule | undefined = undefined
</script>

<div class="grid grid-cols-3 w-full h-full">
	<div class="h-full col-span-2 w-full border border-gray-700" class:overflow-auto={overflowAuto}>
		<FlowGraph
			modules={flow?.value?.modules}
			failureModule={flow?.value?.failure_module}
			on:click={(e) => (stepDetail = e.detail)}
		/>
	</div>
	<div class="w-full border-r border-b border-t border-gray-700 min-h-[150px] p-2 overflow-auto">
		{#if stepDetail == undefined}
			<span class="font-black text-lg w-full my-4">
				<span>Click on a step to see its details</span>
			</span>
		{:else}
			<div class="font-black text-lg w-full mb-6"
				>Step {stepDetail.id ?? ''}<span class="ml-2 font-normal">{stepDetail.summary || ''}</span
				></div
			>
			{#if stepDetail.value.type == 'identity'}
				<div> An identity step return as output its input </div>
			{:else if stepDetail.value.type == 'rawscript'}
				<div class="text-2xs mb-4">
					<h3 class="mb-2">Step Inputs</h3>
					<InputTransformsViewer
						inputTransforms={stepDetail?.value?.input_transforms ??
							stepDetail?.input_transforms ??
							{}}
					/>
				</div>

				<h3 class="mb-2">Code</h3>
				<HighlightCode language={stepDetail.value.language} code={stepDetail.value.content} />
			{:else if stepDetail.value.type == 'script'}
				<div class="mb-4">
					<a
						rel="noreferrer"
						target="_blank"
						href={scriptPathToHref(stepDetail?.value?.path ?? '')}
						class=""
					>
						<IconedPath path={stepDetail?.value?.path ?? ''} />
					</a>
				</div>
				<div class="text-2xs mb-4">
					<h3 class="mb-2">Step Inputs</h3>
					<InputTransformsViewer
						inputTransforms={stepDetail?.value?.input_transforms ??
							stepDetail?.input_transforms ??
							{}}
					/>
				</div>
				{#if stepDetail.value.path.startsWith('hub/')}
					<div class="mt-6">
						<h3>Code</h3>
						<iframe
							class="w-full h-full  text-sm"
							title="embedded script from hub"
							frameborder="0"
							src="https://hub.windmill.dev/embed/script/{stepDetail.value?.path?.substring(4)}"
						/>
					</div>
				{/if}
			{/if}
		{/if}
	</div>
</div>
