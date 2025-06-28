<script lang="ts">
	import HighlightCode from './HighlightCode.svelte'
	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'
	import type { FlowModule } from '$lib/gen'
	import { Badge, Button, Drawer, DrawerContent } from './common'
	import { Highlight } from 'svelte-highlight'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import typescript from 'svelte-highlight/languages/typescript'
	import FlowPathViewer from './flows/content/FlowPathViewer.svelte'
	import SchemaViewer from './SchemaViewer.svelte'
	import { scriptPathToHref } from '$lib/scripts'
	import { cleanExpr, copyToClipboard } from '$lib/utils'
	import { hubBaseUrlStore } from '$lib/stores'

	import { twMerge } from 'tailwind-merge'
	import FlowModuleScript from './flows/content/FlowModuleScript.svelte'
	import { Copy } from 'lucide-svelte'
	import HighlightTheme from './HighlightTheme.svelte'

	export let schema: any | undefined = undefined

	export let stepDetail: FlowModule | string | undefined = undefined
	let codeViewer: Drawer
</script>

<HighlightTheme />

<Drawer bind:this={codeViewer} size="900px">
	<DrawerContent title={'Expanded Code'} on:close={codeViewer.closeDrawer}>
		{#if stepDetail && typeof stepDetail != 'string'}
			{#if stepDetail.value.type == 'script'}
				<div class="mb-4">
					<a
						rel="noreferrer"
						target="_blank"
						href={scriptPathToHref(stepDetail?.value?.path ?? '', $hubBaseUrlStore)}
						class=""
					>
						<IconedPath path={stepDetail?.value?.path ?? ''} />
					</a>
				</div>
				<div class="text-2xs mb-4 mt-2">
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
							src="{$hubBaseUrlStore}/embed/script/{stepDetail.value?.path?.substring(4)}"
						></iframe>
					</div>
				{/if}
			{:else if stepDetail.value.type == 'rawscript'}
				<div class="text-2xs mb-4 mt-2">
					<h3 class="mb-2">Step Inputs</h3>
					<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
				</div>

				<h3 class="mb-2">Code</h3>
				<span class="!text-xs">
					<HighlightCode language={stepDetail.value.language} code={stepDetail.value.content} />
				</span>
				<h3 class="mt-4 mb-2">Lockfile</h3>
				<div>
					{#if stepDetail.value.lock}
						<pre class="bg-surface-secondary text-sm p-2 h-full overflow-auto w-full"
							>{stepDetail.value.lock}</pre
						>
					{:else}
						<p class="bg-surface-secondary text-sm p-2"> There is no lock file for this script </p>
					{/if}
				</div>
			{/if}
		{/if}
	</DrawerContent>
</Drawer>

<div class={twMerge('p-2 overflow-y-scroll')}>
	{#if stepDetail == undefined}
		<div>
			<p class="font-medium text-secondary text-center pt-4 pb-8">
				Click on a step to see its details
			</p>
			{#if schema}
				<h3 class="mb-2 font-semibold">Flow Inputs</h3>
				<SchemaViewer {schema} />
			{/if}
		</div>
	{:else if stepDetail == 'Input'}
		{#if schema}
			<SchemaViewer {schema} />
		{:else}
			<p class="font-medium text-secondary text-center pt-4 pb-8"> No input schema </p>
		{/if}
	{:else if stepDetail == 'Result'}
		<p class="font-medium text-secondary text-center pt-4 pb-8"> End of the flow </p>
	{:else if typeof stepDetail != 'string' && stepDetail.value}
		<div class="">
			<div class="sticky top-0 bg-surface w-full flex items-center py-2">
				{#if stepDetail.id && stepDetail.id != 'failure' && stepDetail.id != 'preprocessor'}
					<Badge color="indigo">
						{stepDetail.id}
					</Badge>
				{/if}
				<span
					class={twMerge(
						'font-medium text-lg',
						stepDetail.id !== 'failure' && stepDetail.id !== 'preprocessor' ? 'ml-2' : ''
					)}
				>
					{#if stepDetail.summary}
						{stepDetail.summary}
					{:else if stepDetail.value.type == 'identity'}
						Identity
					{:else if stepDetail.value.type == 'forloopflow'}
						For loop {#if stepDetail.value.parallel}(parallel){/if}
						{#if stepDetail.value.skip_failures}(skip failures){/if}
					{:else if stepDetail.value.type == 'branchall'}
						Run all branches {#if stepDetail.value.parallel}(parallel){/if}
					{:else if stepDetail.value.type == 'branchone'}
						Run one branch
					{:else if stepDetail.value.type == 'flow'}
						Inner flow
					{:else if stepDetail.value.type == 'whileloopflow'}
						While loop
					{:else if stepDetail.id === 'failure'}
						Error handler
					{:else if stepDetail.id === 'preprocessor'}
						Preprocessor
					{:else if stepDetail.value.type == 'rawscript'}
						Inline {stepDetail.value.language} script
					{:else if stepDetail.value.type == 'script'}
						Workspace script
					{/if}
				</span>
			</div>
			{#if stepDetail.value.type == 'script'}
				<div class="pb-2">
					<a
						rel="noreferrer"
						target="_blank"
						href={scriptPathToHref(stepDetail?.value?.path ?? '', $hubBaseUrlStore)}
						class=""
					>
						<IconedPath path={stepDetail?.value?.path ?? ''} />
					</a>
				</div>
			{/if}
		</div>
		{#if stepDetail.value.type == 'identity'}
			<p class="font-medium text-secondary text-center pt-4 pb-8">
				An identity step returns its inputs as outputs
			</p>
		{:else if stepDetail.value.type == 'rawscript'}
			{#if stepDetail.id !== 'preprocessor'}
				<div class="text-xs">
					<h3 class="mb-2 font-semibold mt-2">Step Inputs</h3>
					<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
				</div>
			{/if}

			<div>
				<div class="mb-2 mt-4 flex justify-between items-center">
					<h3 class="font-semibold">Code</h3>
					<Button size="xs2" color="light" variant="contained" on:click={codeViewer.openDrawer}>
						Expand
					</Button>
				</div>
				<div class="border p-2 rounded-md">
					<HighlightCode
						language={stepDetail.value.language}
						code={stepDetail.value.content}
						class="whitespace-pre-wrap"
					/>
				</div>
				<h3 class="mb-2 mt-4">Lockfile</h3>
				<div>
					{#if stepDetail.value.lock}
						<pre class="bg-surface-secondary text-xs p-2 h-full overflow-auto w-full"
							>{stepDetail.value.lock}</pre
						>
					{:else}
						<p class="bg-surface-secondary text-sm p-2 rounded">
							There is no lockfile for this inline script
						</p>
					{/if}
				</div>
			</div>
		{:else if stepDetail.value.type == 'script'}
			{#if stepDetail.id !== 'preprocessor'}
				<div class="text-2xs">
					<h3 class="mb-2 font-semibold mt-2">Step Inputs</h3>
					<InputTransformsViewer inputTransforms={stepDetail?.value?.input_transforms ?? {}} />
				</div>
			{/if}
			{#if stepDetail.value.path.startsWith('hub/')}
				<div class="flex flex-col grow">
					<div class="mb-2 flex justify-between items-center">
						<h3 class="font-semibold">Code</h3>
						<Button size="xs2" color="light" variant="contained" on:click={codeViewer.openDrawer}>
							Expand
						</Button>
					</div>
					<iframe
						class="w-full grow text-sm h-full"
						title="embedded script from hub"
						frameborder="0"
						src="{$hubBaseUrlStore}/embed/script/{stepDetail.value?.path?.substring(4)}"
					></iframe>
				</div>
			{:else}
				<FlowModuleScript path={stepDetail.value.path} />
			{/if}
		{:else if stepDetail.value.type == 'forloopflow'}
			<div>
				<p class="font-medium text-secondary pb-2"> Iterator expression: </p>
				{#if stepDetail.value.iterator.type == 'static'}
					<ObjectViewer json={stepDetail.value.iterator.value} />
				{:else}
					<span class="text-xs">
						<Highlight language={typescript} code={cleanExpr(stepDetail.value.iterator.expr)} />
					</span>
				{/if}
			</div>
		{:else if stepDetail.value.type == 'whileloopflow'}
			<div>
				{#if stepDetail.stop_after_if}
					<p class="font-medium text-secondary pb-2 pt-4">Stop after if expr:: </p>
					<span class="text-xs">
						<Highlight language={typescript} code={cleanExpr(stepDetail.stop_after_if?.expr)} />
					</span>
				{/if}
			</div>
			<div>
				{#if stepDetail.stop_after_all_iters_if}
					<p class="font-medium text-secondary pb-2 pt-4">Stop after all iters if expr:: </p>
					<span class="text-xs">
						<Highlight
							language={typescript}
							code={cleanExpr(stepDetail.stop_after_all_iters_if?.expr)}
						/>
					</span>
				{/if}
			</div>
		{:else if stepDetail.value.type == 'branchall'}
			<p class="font-medium text-secondary text-center pt-4 pb-8">
				All branches will run, regardless of the inputs
			</p>
		{:else if stepDetail.value.type == 'branchone'}
			<p class="font-medium text-secondary text-center pt-4 pb-8">
				Only one branch will run based on a predicate
			</p>
			<div class="flex-col flex gap-2">
				<div class="flex flex-row gap-4 text-sm p-2">
					<Badge large={true} color="blue">Default branch</Badge>
					<p class="italic text-tertiary"
						>If none of the predicates' expressions evaluated in-order match, this branch is chosen</p
					>
				</div>
				{#each stepDetail.value.branches as v, i}
					<div class="flex flex-col gap-4-2 items-center">
						<div class="w-full flex gap-2 px-2 pt-4 pb-2">
							<Badge large={true} color="blue">Branch {i + 1}</Badge>
							<span>{v.summary}</span>
						</div>
						<div class="w-full border p-2">
							<HighlightCode language="frontend" code={v.expr} />
						</div>
					</div>
				{/each}
			</div>
		{:else if stepDetail.value.type == 'flow'}
			<div class="text-sm mb-1 flex justify-end flex-row">
				<Button
					size="xs2"
					startIcon={{ icon: Copy }}
					color="light"
					on:click={() => {
						if (typeof stepDetail !== 'string') {
							const val = stepDetail?.value
							if (val?.type == 'flow') {
								copyToClipboard(val.path)
							}
						}
					}}
				>
					Copy path
				</Button>
			</div>
			<FlowPathViewer noSide path={stepDetail.value.path} />
		{/if}
	{:else}
		<p class="font-medium text-secondary text-center pt-4 pb-8">
			Step {stepDetail} selected
		</p>
	{/if}
</div>
