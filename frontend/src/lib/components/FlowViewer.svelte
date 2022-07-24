<script lang="ts">
	import { scriptPathToHref } from '$lib/utils'

	import Highlight from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import python from 'svelte-highlight/languages/python'
	import typescript from 'svelte-highlight/languages/typescript'

	import { FlowModuleValue, type FlowValue } from '$lib/gen'
	import github from 'svelte-highlight/styles/github'
	import { slide } from 'svelte/transition'
	import Tabs from './Tabs.svelte'

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

	export let tab: 'ui' | 'json' = 'ui'
	let open: { [id: number]: boolean } = {}
</script>

<svelte:head>
	{@html github}
</svelte:head>

{#if !embedded}
	<Tabs
		tabs={[
			['ui', 'Flow rendered'],
			['json', 'JSON']
		]}
		bind:tab
	/>
{/if}
{#if tab == 'ui'}
	<div class="flow-root w-full p-4">
		<p class="font-black text-lg mb-6 w-full ml-2">
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
										{#if mod?.value?.type == FlowModuleValue.type.SCRIPT}
											Script at path <a
												target="_blank"
												href={scriptPathToHref(mod?.value?.path ?? '')}
												class="font-medium text-gray-900">{mod?.value?.path}</a
											>
										{:else if mod?.value?.type == FlowModuleValue.type.RAWSCRIPT}
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
										{:else if mod?.value?.type == FlowModuleValue.type.FLOW}
											Flow at path {mod?.value?.path}
										{:else if mod?.value?.type == FlowModuleValue.type.FORLOOPFLOW}
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
{:else}
	<Highlight language={json} code={JSON.stringify(flowFiltered, null, 4)} />
{/if}
