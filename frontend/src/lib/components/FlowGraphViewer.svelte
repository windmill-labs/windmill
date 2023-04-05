<script lang="ts">
	import { FlowGraph } from './graph'
	import HighlightCode from './HighlightCode.svelte'
	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'
	import { scriptPathToHref } from '../utils'
	import type { FlowModule, FlowValue } from '$lib/gen'
	import { Drawer, DrawerContent } from './common'
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
						<h3>Code</h3>
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
<div class="grid grid-cols-3 w-full">
	<div
		class="{noSide
			? 'col-span-3'
			: 'sm:col-span-2 col-span-3'} w-full border border-gray-400 max-h-screen"
		class:overflow-auto={overflowAuto}
	>
		<FlowGraph
			minHeight={400}
			modules={flow?.value?.modules}
			failureModule={flow?.value?.failure_module}
			on:select={(e) => (stepDetail = e.detail)}
		/>
	</div>
	{#if !noSide}
		<div
			class="w-full border-r border-b border-t border-gray-400 min-h-[150px] p-2 overflow-auto hidden sm:block"
		>
			{#if stepDetail == undefined}
				<SchemaViewer schema={flow?.schema} />

				<span class="font-black text-lg w-full my-4 mt-14">
					<span>Click on a step to see its details</span>
				</span>
			{:else if stepDetail == 'Input'}
				<SchemaViewer schema={flow?.schema} />
			{:else if stepDetail == 'Result'}
				End of the flow
			{:else if typeof stepDetail != 'string' && stepDetail.value}
				<div class="font-black text-lg w-full mb-6"
					>Step {stepDetail.id ?? ''}<span class="ml-2 font-normal">{stepDetail.summary || ''}</span
					></div
				>
				{#if stepDetail.value.type == 'identity'}
					<div> An identity step return as output its input </div>
				{:else if stepDetail.value.type == 'rawscript'}
					<div class="text-2xs mb-4">
						<h3 class="mb-2">Step Inputs</h3>
						<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
					</div>

					<h3 class="mb-2"
						>Code &nbsp; <button class="text-gray-600 text-xs" on:click={codeViewer.openDrawer}
							>Expand</button
						>
					</h3>
					<span class="!text-xs">
						<HighlightCode language={stepDetail.value.language} code={stepDetail.value.content} />
					</span>
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
						<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
					</div>
					{#if stepDetail.value.path.startsWith('hub/')}
						<div class="mt-6">
							<h3
								>Code&nbsp; <button class="text-gray-600 text-xs" on:click={codeViewer.openDrawer}
									>Expand</button
								>
							</h3>
							<iframe
								class="w-full h-full text-sm"
								title="embedded script from hub"
								frameborder="0"
								src="https://hub.windmill.dev/embed/script/{stepDetail.value?.path?.substring(4)}"
							/>
						</div>
					{/if}
				{:else if stepDetail.value.type == 'forloopflow'}
					<p
						>For loop with iterator: {#if stepDetail.value.iterator.type == 'static'}<ObjectViewer
								json={stepDetail.value.iterator.value}
							/>{:else}
							<span class="text-xs">
								<Highlight language={typescript} code={cleanExpr(stepDetail.value.iterator.expr)} />
							</span>
						{/if}</p
					>
				{:else if stepDetail.value.type == 'branchall'}
					<p>Run all branches</p>
				{:else if stepDetail.value.type == 'branchone'}
					<p>Run one branch based on a predicate</p>
				{:else if stepDetail.value.type == 'flow'}
					<FlowPathViewer noSide path={stepDetail.value.path} />
				{/if}
			{/if}
		</div>
	{/if}
</div>
