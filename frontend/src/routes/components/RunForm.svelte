<script lang="ts">
	import { page } from '$app/stores'
	import type { Script, Flow } from '../../gen'
	import { getToday } from '../../utils'
	import { slide } from 'svelte/transition'

	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import Tooltip from './Tooltip.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import SchemaForm from './SchemaForm.svelte'

	export let runnable: Script | Flow | undefined
	export let runAction: (scheduledForStr: string | undefined, args: Record<string, any>) => void
	export let buttonText = 'Run'
	export let schedulable = true
	export let detailed = true

	export let args: Record<string, any> = {}

	let isValid = true

	let queryArgs = $page.url.searchParams.get('args')
	if (queryArgs) {
		const parsed = JSON.parse(atob(queryArgs))
		Object.entries(parsed).forEach(([k, v]) => {
			if (v == '<function call>') {
				parsed[k] = undefined
			}
		})
		console.log(parsed)
		args = parsed
	}

	// Run later
	let viewOptions = false
	let scheduledForStr: string | undefined
</script>

<div class="max-w-5xl">
	{#if detailed}
		<div class="grid grid-cols-3 gap-2">
			<div>
				<h2 class="mb-1">Summary</h2>
				<div class="mb-2 md:mb-3 text-sm">
					{runnable?.summary ? runnable?.summary : 'No summary'}
				</div>
			</div>
			<div class="col-span-2">
				<h2 class="mb-1">Description</h2>
				<div class="mb-2 md:mb-6">
					<div class="prose text-sm">
						<SvelteMarkdown
							source={runnable?.description ? runnable?.description : 'No description'}
						/>
					</div>
				</div>
			</div>
		</div>
	{/if}
	{#if runnable?.schema}
		{#if detailed}
			<h2>
				Arguments<Tooltip class="px-2 mb-6"
					>Optioal fields, if left blank, will use the placeholder value as default.</Tooltip
				>
			</h2>
		{/if}
		{#if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm">No arguments</div>
		{:else}
			<div
				class="bg-gray-50 border border shadow-md shadow-blue-100 shadow-inner rounded border-gray-300 p-6"
			>
				<SchemaForm schema={runnable.schema} bind:isValid bind:args />
			</div>
		{/if}
	{:else}
		<div class="text-sm">No schema</div>
	{/if}
	{#if viewOptions}
		<div transition:slide class="mt-6">
			<h2>Run later</h2>
			<div class="border rounded-md p-3 pt-4">
				<div class="flex flex-row items-end">
					<div class="w-max md:w-2/3 mt-2 mb-1">
						<label for="run-time" />
						<input
							class="inline-block"
							type="datetime-local"
							id="run-time"
							name="run-scheduled-time"
							bind:value={scheduledForStr}
							min={getToday().toISOString().slice(0, 16)}
						/>
					</div>
					<button
						class="default-button-secondary mx-2 mb-1"
						on:click={() => {
							scheduledForStr = undefined
						}}>clear</button
					>
				</div>
			</div>
		</div>
	{/if}
	<div class="flex justify-between mt-2 md:mt-6 mb-6">
		<button
			type="submit"
			class="mr-6 text-sm underline text-gray-700 inline-flex  items-center"
			on:click={() => {
				viewOptions = !viewOptions
			}}
		>
			{#if schedulable}
				<div>
					schedule to run later <Icon
						data={viewOptions ? faChevronUp : faChevronDown}
						scale={0.5}
					/>
				</div>
			{/if}
		</button>
		<button
			type="submit"
			disabled={!isValid}
			class="{isValid ? 'default-button' : 'default-button-disabled'} w-min px-6"
			on:click={() => {
				runAction(scheduledForStr, args)
			}}
		>
			{scheduledForStr ? 'Schedule run to a later time' : buttonText}
		</button>
	</div>
</div>
