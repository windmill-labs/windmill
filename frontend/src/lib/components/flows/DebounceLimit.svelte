<script lang="ts">
	import Toggle from '$lib/components/Toggle.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import { enterpriseLicense } from '$lib/stores'
	import Label from '../Label.svelte'
	import type { Schema } from '$lib/common'

	let {
		debounce_delay_s = $bindable(),
		debounce_key = $bindable(),
		debounce_args_to_accumulate = $bindable(),
		max_total_debouncing_time = $bindable(),
		max_total_debounces_amount = $bindable(),
		schema = undefined,
		placeholder,
		size = 'xs',
		color = undefined,
		fontClass = 'font-normal'
	}: {
		debounce_delay_s: number | undefined
		debounce_key: string | undefined
		debounce_args_to_accumulate: string[] | undefined
		max_total_debouncing_time: number | undefined
		max_total_debounces_amount: number | undefined
		schema?: Schema
		placeholder: string
		size: 'xs' | 'sm'
		color?: 'nord' | undefined
		fontClass?: string
	} = $props()

	// Get list of array-type arguments from schema
	let arrayArgs = $derived(
		schema?.properties
			? Object.entries(schema.properties)
					.filter(([_, prop]) => prop.type === 'array')
					.map(([key, _]) => key)
			: []
	)

	// Single selected arg for UI (convert between single selection and array)
	let selectedArg = $state(debounce_args_to_accumulate?.[0] ?? '')

	$effect(() => {
		if (selectedArg && selectedArg !== '') {
			debounce_args_to_accumulate = [selectedArg]
		} else {
			debounce_args_to_accumulate = undefined
		}
	})

	// Convert 0 to undefined for max values
	$effect(() => {
		if (max_total_debouncing_time === 0) {
			max_total_debouncing_time = undefined
		}
		if (max_total_debounces_amount === 0) {
			max_total_debounces_amount = undefined
		}
	})
</script>

<div>
	<div class="flex flex-row items-center gap-2">
		<Toggle
			textClass={fontClass}
			{color}
			{size}
			disabled={!$enterpriseLicense}
			checked={Boolean(debounce_delay_s)}
			on:change={() => {
				if (debounce_delay_s) {
					debounce_delay_s = undefined
					debounce_key = undefined
				} else {
					debounce_delay_s = 1
				}
			}}
			options={{
				right: 'Debouncing',
				rightTooltip: 'Consolidate multiple executions into a single run within a time window',
				rightDocumentationLink: 'https://www.windmill.dev/docs/core_concepts/job_debouncing'
			}}
			class="py-1"
			eeOnly={true}
		/>
	</div>

	{#if debounce_delay_s}
		<div class="flex flex-col gap-4 mt-2">
			<Label label="Delay in seconds">
				<SecondsInput disabled={!$enterpriseLicense} bind:seconds={debounce_delay_s} />
			</Label>
			<Label label="Custom debounce key (optional)">
				{#snippet header()}
					<Tooltip>
						Debounce keys are global, you can have them be workspace specific using the variable
						`$workspace`. You can also use an argument's value using `$args[name_of_arg]`</Tooltip
					>
				{/snippet}
				<!-- svelte-ignore a11y_autofocus -->
				<input
					type="text"
					autofocus
					disabled={!$enterpriseLicense}
					bind:value={debounce_key}
					{placeholder}
				/>
			</Label>
			<Label label="Argument to accumulate (optional)">
				{#snippet header()}
					<Tooltip>
						Select a list-type argument to accumulate across debounced executions. Values from
						each debounced execution will be appended together.</Tooltip
					>
				{/snippet}
				<select disabled={!$enterpriseLicense} bind:value={selectedArg}>
					<option value="">None</option>
					{#each arrayArgs as arg}
						<option value={arg}>{arg}</option>
					{/each}
				</select>
				{#if arrayArgs.length === 0}
					<div class="text-xs text-gray-500 mt-1">
						No list-type arguments found in schema. Define arguments with type "array" to enable
						accumulation.
					</div>
				{/if}
			</Label>
			<Label label="Max total debouncing time (optional)">
				{#snippet header()}
					<Tooltip>
						Maximum total time (in seconds) that a job can be debounced before it must execute.
						Once this time is reached, the job will run regardless of ongoing debouncing.</Tooltip
					>
				{/snippet}
				<SecondsInput disabled={!$enterpriseLicense} bind:seconds={max_total_debouncing_time} />
			</Label>
			<Label label="Max total debounces amount (optional)">
				{#snippet header()}
					<Tooltip>
						Maximum number of times a job can be debounced before it must execute. Once this
						count is reached, the job will run regardless of ongoing debouncing.</Tooltip
					>
				{/snippet}
				<input type="number" disabled={!$enterpriseLicense} bind:value={max_total_debounces_amount} min="0" />
			</Label>
		</div>
	{/if}
</div>
