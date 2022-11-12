<script lang="ts">
	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { faClipboard } from '@fortawesome/free-solid-svg-icons'
	import type { FlowModule, FlowValue } from '$lib/gen'
	import { Tab, Tabs, TabContent, Button } from './common'
	import SchemaViewer from './SchemaViewer.svelte'
	import FieldHeader from './FieldHeader.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { copyToClipboard, scriptPathToHref } from '../utils'
	import { FlowGraph } from './graph'
	import HighlightCode from './HighlightCode.svelte'
	import InputTransformsViewer from './InputTransformsViewer.svelte'
	import IconedPath from './IconedPath.svelte'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}
	export let initialOpen: number | undefined = undefined

	$: flowFiltered = {
		summary: flow.summary,
		description: flow.description,
		value: flow.value,
		schema: flow.schema
	}

	export let tab: 'ui' | 'json' | 'schema' = 'ui'
	export let noSummary = false

	let stepDetail: FlowModule | undefined = undefined

	let open: { [id: number]: boolean } = {}
	if (initialOpen) {
		open[initialOpen] = true
	}

	function toAny(x: unknown): any {
		return x as any
	}
</script>

<Tabs bind:selected={tab}>
	<Tab value="ui">Flow rendered</Tab>
	<Tab value="json">Json</Tab>
	<Tab value="schema">Input schema of the flow</Tab>

	<svelte:fragment slot="content">
		<TabContent value="ui">
			<div class="flow-root w-full pb-4">
				{#if !noSummary}
					<h2 class="my-4">{flow.summary}</h2>
					<SvelteMarkdown source={flow.description ?? ''} />
				{/if}

				<p class="font-black text-lg w-full my-4">
					<span>Flow Input</span>
				</p>
				{#if flow.schema && flow.schema.properties && Object.keys(flow.schema.properties).length > 0 && flow.schema}
					<ul class="my-2">
						{#each Object.entries(flow.schema.properties) as [inp, v]}
							<li class="list-disc flex flex-row">
								<FieldHeader
									label={inp}
									required={flow.schema.required?.includes(inp)}
									type={toAny(v)?.type}
									contentEncoding={toAny(v)?.contentEncoding}
									format={toAny(v)?.format}
									itemsType={toAny(v)?.itemsType}
								/><span class="ml-4 mt-2 text-xs"
									>{toAny(v)?.default != undefined
										? 'default: ' + JSON.stringify(toAny(v)?.default)
										: ''}</span
								>
							</li>
						{/each}
					</ul>
				{:else}
					<div class="text-gray-700 text-xs italic mb-4">No inputs</div>
				{/if}
				<div class="pr-40 flex flex-col">
					<div class="mt-5 h-full w-full border border-gray-700">
						<FlowGraph
							modules={flow?.value?.modules}
							failureModule={flow?.value?.failure_module}
							on:click={(e) => (stepDetail = e.detail)}
						/>
					</div>
					<div class="w-full border-l border-r border-b border-gray-700 min-h-[150px] p-2">
						{#if stepDetail == undefined}
							<span class="font-black text-lg w-full my-4">
								<span>Click on a step to see its details</span>
							</span>
						{:else}
							<div class="font-black text-lg w-full mb-2 "
								>Step {stepDetail.id ?? ''}<span class="ml-2 font-normal"
									>{stepDetail.summary || ''}</span
								></div
							>
							{#if stepDetail.value.type == 'identity'}
								<div> An identity step return as output its input </div>
							{:else if stepDetail.value.type == 'rawscript'}
								<div class="text-2xs mb-4">
									<h3>Step Inputs</h3>
									<InputTransformsViewer
										inputTransforms={stepDetail?.value?.input_transforms ??
											stepDetail?.input_transforms ??
											{}}
									/>
								</div>

								<h3>Code</h3>
								<HighlightCode
									language={stepDetail.value.language}
									code={stepDetail.value.content}
								/>
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
									<h3>Step Inputs</h3>
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
											src="https://hub.windmill.dev/embed/script/{stepDetail.value?.path?.substring(
												4
											)}"
										/>
									</div>
								{/if}
							{/if}
						{/if}
					</div>
				</div>
			</div>
		</TabContent>
		<TabContent value="json">
			<div class="relative">
				<Button
					on:click={() => copyToClipboard(JSON.stringify(flowFiltered, null, 4))}
					color="dark"
					variant="border"
					size="sm"
					startIcon={{ icon: faClipboard }}
					btnClasses="absolute top-2 right-2"
				>
					Copy content
				</Button>
				<Highlight language={json} code={JSON.stringify(flowFiltered, null, 4)} />
			</div>
		</TabContent>
		<TabContent value="schema">
			<div class="my-4" />
			<SchemaViewer schema={flow.schema} />
		</TabContent>
	</svelte:fragment>
</Tabs>
