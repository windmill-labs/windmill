<script lang="ts">
	import { FlowGraph } from './graph'
	import HighlightCode from './HighlightCode.svelte'
	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'
	import { scriptPathToHref } from '../utils'
	import type { FlowModule, FlowValue } from '$lib/gen'
	import { Badge, Button, Drawer, DrawerContent } from './common'
	import { Highlight } from 'svelte-highlight'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import typescript from 'svelte-highlight/languages/typescript'
	import { cleanExpr } from './flows/utils'
	import FlowPathViewer from './flows/content/FlowPathViewer.svelte'
	import SchemaViewer from './SchemaViewer.svelte'
	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let overflowAuto = false
	export let noSide = false
	export let download = false

	let stepDetail: FlowModule | string | undefined = undefined
	let codeViewer: Drawer
</script>

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={'Expanded Code'} on:close={codeViewer.closeDrawer}>
		{#if stepDetail && typeof stepDetail != 'string'}
			{#if stepDetail.value.type == 'script'}
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
					<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
				</div>
				{#if stepDetail.value.path.startsWith('hub/')}
					<div class="mt-6">
						<h3 class="mb-2">Code</h3>
						<iframe
							class="w-full h-full text-sm"
							title="embedded script from hub"
							frameborder="0"
							src="https://hub.windmill.dev/embed/script/{stepDetail.value?.path?.substring(4)}"
						/>
					</div>
				{/if}
			{:else if stepDetail.value.type == 'rawscript'}
				<div class="text-2xs mb-4">
					<h3 class="mb-2">Step Inputs</h3>
					<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
				</div>

				<h3 class="mb-2">Code</h3>
				<span class="!text-xs">
					<HighlightCode language={stepDetail.value.language} code={stepDetail.value.content} />
				</span>
			{/if}
		{/if}
	</DrawerContent>
</Drawer>
<div class="grid grid-cols-3 w-full h-full">
	<div
		class="{noSide
			? 'col-span-3'
			: 'sm:col-span-2 col-span-3'} w-full border border-gray-400 max-h-full"
		class:overflow-auto={overflowAuto}
	>
		<FlowGraph
			{download}
			minHeight={400}
			modules={flow?.value?.modules}
			failureModule={flow?.value?.failure_module}
			on:select={(e) => (stepDetail = e.detail)}
		/>
	</div>
	{#if !noSide}
		<div
			class="relative w-full h-full min-h-[150px] max-h-[90vh] border-r border-b border-t border-gray-400
			p-2 pt-0 overflow-auto hidden sm:flex flex-col gap-4"
		>
			{#if stepDetail == undefined}
				<div>
					<p class="font-medium text-gray-600 text-center pt-4 pb-8">
						Click on a step to see its details
					</p>
					<h3 class="mb-2 font-semibold">Flow Inputs</h3>
					<SchemaViewer schema={flow?.schema} />
				</div>
			{:else if stepDetail == 'Input'}
				<SchemaViewer schema={flow?.schema} />
			{:else if stepDetail == 'Result'}
				<p class="font-medium text-gray-600 text-center pt-4 pb-8"> End of the flow </p>
			{:else if typeof stepDetail != 'string' && stepDetail.value}
				<div class="">
					<div class="sticky top-0 bg-white w-full flex items-center py-2">
						{#if stepDetail.id}
							<Badge color="indigo">
								{stepDetail.id}
							</Badge>
						{/if}
						<span class="ml-2 font-medium text-lg">
							{#if stepDetail.summary}
								{stepDetail.summary}
							{:else if stepDetail.value.type == 'identity'}
								Identity
							{:else if stepDetail.value.type == 'forloopflow'}
								For loop
							{:else if stepDetail.value.type == 'branchall'}
								Run all branches
							{:else if stepDetail.value.type == 'branchone'}
								Run one branch
							{:else if stepDetail.value.type == 'flow'}
								Inner flow
							{:else}
								Anonymous step
							{/if}
						</span>
					</div>
					{#if stepDetail.value.type == 'script'}
						<div class="pb-2">
							<a
								rel="noreferrer"
								target="_blank"
								href={scriptPathToHref(stepDetail?.value?.path ?? '')}
								class=""
							>
								<IconedPath path={stepDetail?.value?.path ?? ''} />
							</a>
						</div>
					{/if}
				</div>
				{#if stepDetail.value.type == 'identity'}
					<p class="font-medium text-gray-600 text-center pt-4 pb-8">
						An identity step returns its inputs as outputs
					</p>
				{:else if stepDetail.value.type == 'rawscript'}
					<div class="text-xs">
						<h3 class="mb-2 font-semibold">Step Inputs</h3>
						<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
					</div>

					<div>
						<div class="mb-2 flex justify-between items-center">
							<h3 class="font-semibold">Code</h3>
							<Button size="xs2" color="light" variant="contained" on:click={codeViewer.openDrawer}>
								Expand
							</Button>
						</div>
						<div class="w-full overflow-auto">
							<HighlightCode language={stepDetail.value.language} code={stepDetail.value.content} />
						</div>
					</div>
				{:else if stepDetail.value.type == 'script'}
					<div class="text-2xs">
						<h3 class="mb-2 font-semibold">Step Inputs</h3>
						<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
					</div>
					{#if stepDetail.value.path.startsWith('hub/')}
						<div class="flex flex-col grow">
							<div class="mb-2 flex justify-between items-center">
								<h3 class="font-semibold">Code</h3>
								<Button
									size="xs2"
									color="light"
									variant="contained"
									on:click={codeViewer.openDrawer}
								>
									Expand
								</Button>
							</div>
							<iframe
								class="w-full grow text-sm"
								title="embedded script from hub"
								frameborder="0"
								src="https://hub.windmill.dev/embed/script/{stepDetail.value?.path?.substring(4)}"
							/>
						</div>
					{/if}
				{:else if stepDetail.value.type == 'forloopflow'}
					<div>
						<p class="font-medium text-gray-600 pb-2"> Iterator expression: </p>
						{#if stepDetail.value.iterator.type == 'static'}
							<ObjectViewer json={stepDetail.value.iterator.value} />
						{:else}
							<span class="text-xs">
								<Highlight language={typescript} code={cleanExpr(stepDetail.value.iterator.expr)} />
							</span>
						{/if}
					</div>
				{:else if stepDetail.value.type == 'branchall'}
					<p class="font-medium text-gray-600 text-center pt-4 pb-8">
						All branches will run, regardless of the inputs
					</p>
				{:else if stepDetail.value.type == 'branchone'}
					<p class="font-medium text-gray-600 text-center pt-4 pb-8">
						Only one branch will run based on a predicate
					</p>
				{:else if stepDetail.value.type == 'flow'}
					<FlowPathViewer noSide path={stepDetail.value.path} />
				{/if}
			{/if}
		</div>
	{/if}
</div>
