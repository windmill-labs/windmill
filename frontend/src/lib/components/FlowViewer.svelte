<script lang="ts">
	import { scriptPathToHref } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'

	import type { FlowValue } from '$lib/gen'
	import { slide } from 'svelte/transition'
	import Tabs from './Tabs.svelte'
	import SchemaViewer from './SchemaViewer.svelte'
	import FieldHeader from './FieldHeader.svelte'

	export let flow: {
		summary: string
		description?: string
		value: FlowValue
		schema?: any
	}

	let flowFiltered = {
		summary: flow.summary,
		description: flow.description,
		value: flow.value,
		schema: flow.schema
	}

	export let embedded = false

	export let tab: 'ui' | 'json' | 'schema' = 'ui'
	let open: { [id: number]: boolean } = {}

	function toAny(x: unknown): any {
		return x as any
	}
</script>

{#if !embedded}
	<Tabs
		tabs={[
			['ui', 'Flow rendered'],
			['json', 'JSON'],
			['schema', 'Input schema of the flow']
		]}
		bind:tab
	/>
{/if}
{#if tab == 'ui'}
	<div class="flow-root w-full p-4">
		<p class="font-black text-lg w-full ml-2">
			<span>Inputs</span>
		</p>
		{#if flow.schema && flow.schema.properties && Object.keys(flow.schema.properties).length > 0 && flow.schema}
			<ul class="my-4 ml-6">
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
			<div class="text-gray-700 text-xs italic mb-4">
				This script has no argument or is ill-defined
			</div>
		{/if}
		<p class="font-black text-lg my-6 w-full ml-2">
			<span>{flow?.value?.modules?.length} Steps </span>
			<span class="mt-4" />
		</p>
		<ul class="-mb-8 w-full">
			{#each flow?.value?.modules ?? [] as mod, i}
				<li class="w-full">
					<div class="relative pb-8 w-full">
						{#if i < (flow?.value?.modules ?? []).length - 1}
							<span
								class="absolute top-4 left-4 -ml-px h-full w-0.5 bg-gray-200"
								aria-hidden="true"
							/>
						{/if}
						<div class="relative flex space-x-3">
							<div>
								<span
									class="h-8 w-8 rounded-full bg-blue-600 flex items-center justify-center ring-8 ring-white text-white"
									>{i + 1}
								</span>
							</div>
							<div class="min-w-0 flex-1 pt-1.5 flex justify-between space-x-4 w-full">
								<div class="w-full">
									<p class="text-sm text-gray-500">
										{#if mod?.value?.type == 'script'}
											Script at path <a
												target="_blank"
												href={scriptPathToHref(mod?.value?.path ?? '')}
												class="font-medium text-gray-900">{mod?.value?.path}</a
											>
										{:else if mod?.value?.type == 'rawscript'}
											<button
												on:click={() => (open[i] = !open[i])}
												class="mb-2 underline text-black"
											>
												Raw {mod?.value?.language} script {open[i] ? '(-)' : '(+)'}</button
											>

											{#if open[i]}
												<div transition:slide class="border border-black p-2 bg-gray-50 w-full">
													<Highlight
														language={mod?.value?.language == 'deno' ? typescript : python}
														code={mod?.value?.content}
													/>
												</div>
											{/if}
										{:else if mod?.value?.type == 'flow'}
											Flow at path {mod?.value?.path}
										{:else if mod?.value?.type == 'forloopflow'}
											For loop over step {i}'s result':
											<svelte:self flow={mod?.value} embedded={true} />
										{/if}
									</p>
								</div>
							</div>
						</div>
					</div>
				</li>
			{/each}
		</ul>
	</div>
{:else if tab == 'json'}
	<div class="relative">
		<button
			on:click={async () => {
				await navigator.clipboard.writeText(JSON.stringify(flowFiltered, null, 4))
			}}
			class="absolute default-secondary-button-v2 bg-white/30 right-0 my-2 ml-4"
			>copy content</button
		>
		<Highlight language={json} code={JSON.stringify(flowFiltered, null, 4)} />
	</div>
{:else if tab == 'schema'}
	<SchemaViewer schema={flow.schema} />
{/if}
