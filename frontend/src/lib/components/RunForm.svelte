<script lang="ts">
	import { page } from '$app/stores'
	import { decodeArgs, decodeState, getToday } from '$lib/utils'
	import { slide } from 'svelte/transition'

	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import SvelteMarkdown from 'svelte-markdown'
	import SchemaForm from './SchemaForm.svelte'
	import Tooltip from './Tooltip.svelte'
	import type { Schema } from '$lib/common'
	import { Button } from './common'

	export let runnable: { summary?: string; schema?: Schema; description?: string } | undefined
	export let runAction: (scheduledForStr: string | undefined, args: Record<string, any>) => void
	export let buttonText = 'Run'
	export let schedulable = true
	export let detailed = true

	export let args: Record<string, any> = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	let isValid = true

	// Run later
	let viewOptions = false
	let scheduledForStr: string | undefined
</script>

<div class="max-w-6xl">
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
				Arguments
				<Tooltip>
					The optional fields, if left blank, will use the placeholder value as default.
				</Tooltip>
			</h2>
		{/if}
		{#if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm p-4">No arguments</div>
		{:else}
			<SchemaForm schema={runnable.schema} bind:isValid bind:args />
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
					<Button
						variant="border"
						color="blue"
						size="sm"
						btnClasses="mx-2 mb-1"
						on:click={() => {
							scheduledForStr = undefined
						}}
					>
						Clear
					</Button>
				</div>
			</div>
		</div>
	{/if}
	{#if schedulable}
		<div class="flex justify-between mt-2 md:mt-6 mb-6">
			<Button
				color="light"
				size="sm"
				endIcon={{ icon: viewOptions ? faChevronUp : faChevronDown }}
				on:click={() => (viewOptions = !viewOptions)}
			>
				Schedule to run later
			</Button>
			<Button
				btnClasses="!px-6 !py-1"
				disabled={!isValid}
				on:click={() => runAction(scheduledForStr, args)}
			>
				{scheduledForStr ? 'Schedule run to a later time' : buttonText}
			</Button>
		</div>
	{:else}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args)}
		>
			{buttonText}
		</Button>
	{/if}
</div>
