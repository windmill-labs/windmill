<script lang="ts">
	import { page } from '$app/stores'
	import {
		decodeArgs,
		defaultIfEmptyString,
		displayDaysAgo,
		emptyString,
		getToday,
		truncateHash
	} from '$lib/utils'
	import { slide } from 'svelte/transition'

	import { faChevronDown, faChevronUp } from '@fortawesome/free-solid-svg-icons'
	import SchemaForm from './SchemaForm.svelte'
	import type { Schema } from '$lib/common'
	import { Badge, Button } from './common'
	import SharedBadge from './SharedBadge.svelte'
	import Toggle from './Toggle.svelte'
	import { userStore } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'

	export let runnable:
		| {
				summary?: string
				schema?: Schema
				description?: string
				path?: string
				is_template?: boolean
				hash?: string
				kind?: string
				can_write?: boolean
				created_at?: string
				created_by?: string
				extra_perms?: Record<string, boolean>
		  }
		| undefined
	export let runAction: (
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisible_to_owner?: boolean
	) => void
	export let buttonText = 'Run'
	export let schedulable = true
	export let detailed = true
	export let autofocus = false
	export let topButton = false
	export let loading = false
	export let noVariablePicker = false

	export let args: Record<string, any> = decodeArgs($page.url.searchParams.get('args') ?? undefined)

	export function run() {
		runAction(scheduledForStr, args, invisible_to_owner)
	}

	export let isValid = true

	// Run later
	let viewOptions = false
	let scheduledForStr: string | undefined
	let invisible_to_owner: false
</script>

<div class="max-w-6xl">
	{#if detailed}
		{#if runnable}
			<div class="flex flex-row flex-wrap justify-between gap-4">
				<div>
					<div class="flex flex-col mb-2">
						<h1 class="break-words py-2 mr-2">
							{defaultIfEmptyString(runnable.summary, runnable.path ?? '')}
						</h1>
						{#if !emptyString(runnable.summary)}
							<h2 class="font-bold pb-4">{runnable.path}</h2>
						{/if}

						<div class="flex items-center gap-2">
							<span class="text-sm text-gray-500">
								{#if runnable}
									Edited {displayDaysAgo(runnable.created_at || '')} by {runnable.created_by ||
										'unknown'}
								{/if}
							</span>
							<Badge color="dark-gray">
								{truncateHash(runnable?.hash ?? '')}
							</Badge>
							{#if runnable?.is_template}
								<Badge color="blue">Template</Badge>
							{/if}
							{#if runnable && runnable.kind !== 'runnable'}
								<Badge color="blue">
									{runnable?.kind}
								</Badge>
							{/if}
							<SharedBadge
								canWrite={runnable.can_write ?? true}
								extraPerms={runnable?.extra_perms ?? {}}
							/>
						</div>
					</div>
				</div>
			</div>
		{:else}
			<h1>Loading...</h1>
		{/if}
	{/if}
	{#if topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args)}
		>
			{buttonText}
		</Button>
	{/if}
	{#if runnable?.schema}
		<div class="my-2" />
		{#if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm p-4">No arguments</div>
		{:else}
			<SchemaForm {noVariablePicker} {autofocus} schema={runnable.schema} bind:isValid bind:args />
		{/if}
	{:else}
		<div class="text-xs text-gray-600">No schema</div>
	{/if}
	{#if schedulable}
		<div class="flex gap-2 items-start flex-wrap justify-between mt-2 md:mt-6 mb-6">
			<div class="flex flex-col">
				<div>
					<Button
						color="light"
						size="sm"
						endIcon={{ icon: viewOptions ? faChevronUp : faChevronDown }}
						on:click={() => (viewOptions = !viewOptions)}
					>
						Schedule to run later
					</Button>
				</div>
				{#if viewOptions}
					<div transition:slide class="mt-6">
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
			</div>
			{#if runnable?.path?.startsWith(`u/${$userStore?.username}`) != true && (runnable?.path?.split('/')?.length ?? 0) > 2}
				<div class="flex items-center gap-1">
					<Toggle
						options={{
							right: `run only visible to you`
						}}
						bind:checked={invisible_to_owner}
					/>
					<Tooltip
						>By default, runs are visible to the owner(s) of the script or flow being triggered</Tooltip
					>
				</div>
			{/if}
			<div class="flex-row-reverse flex grow">
				<Button
					{loading}
					btnClasses="!px-6 !py-1"
					disabled={!isValid}
					on:click={() => runAction(scheduledForStr, args, invisible_to_owner)}
				>
					{scheduledForStr ? 'Schedule to run later' : buttonText}
				</Button>
			</div>
		</div>
	{:else if !topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args, invisible_to_owner)}
		>
			{buttonText}
		</Button>
	{/if}
</div>
