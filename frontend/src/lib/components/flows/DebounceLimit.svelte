<script lang="ts">
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { SecondsInput } from '$lib/components/common'
	import { enterpriseLicense } from '$lib/stores'
	import Label from '../Label.svelte'
	import type { Schema } from '$lib/common'
	import Select from '$lib/components/select/Select.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import SettingToggle from '$lib/components/itemSettings/SettingToggle.svelte'

	let {
		debounce_delay_s = $bindable(),
		debounce_key = $bindable(),
		debounce_args_to_accumulate = $bindable(),
		max_total_debouncing_time = $bindable(),
		max_total_debounces_amount = $bindable(),
		schema = undefined,
		placeholder
	}: {
		debounce_delay_s: number | undefined
		debounce_key: string | undefined
		debounce_args_to_accumulate: string[] | undefined
		max_total_debouncing_time: number | undefined
		max_total_debounces_amount: number | undefined
		schema?: Schema
		placeholder: string
	} = $props()

	// Check if an originalType like "string | string[]" is a top-level
	// union where at least one member is an array type (ends with "[]").
	// Splits on "|" only at the top level (not inside {}, <>, or ()).
	function isUnionWithArray(originalType: string | undefined): boolean {
		if (!originalType) return false
		let depth = 0
		const parts: string[] = []
		let cur = ''
		for (const ch of originalType) {
			if (ch === '{' || ch === '<' || ch === '(') depth++
			else if (ch === '}' || ch === '>' || ch === ')') depth--
			else if (ch === '|' && depth === 0) {
				parts.push(cur.trim())
				cur = ''
				continue
			}
			cur += ch
		}
		parts.push(cur.trim())
		// Match TS array syntax (T[]) and Python list syntax (list[T] / List[T])
		return parts.length > 1 && parts.some((p) => p.endsWith('[]') || /^[Ll]ist\[.+\]$/.test(p))
	}

	// Get list of arguments eligible for accumulation from schema.
	// Includes array-type arguments and union types like T | T[]
	// whose scalar values are wrapped into single-element arrays
	// at aggregation time.
	let arrayArgs = $derived(
		schema?.properties
			? Object.entries(schema.properties)
					.filter(([_, prop]) => prop.type === 'array' || isUnionWithArray(prop.originalType))
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

	let disabled = $derived(!$enterpriseLicense)
</script>

<!-- Presence, not truthiness: a single backspace takes the seeded 1s to 0 while the user is
     still typing, and reading that as off would close the fields mid-edit. -->
<SettingToggle
	setting="debounce"
	label="Debouncing"
	tooltip="Consolidate multiple executions into a single run within a time window"
	documentationLink="https://www.windmill.dev/docs/core_concepts/job_debouncing"
	eeOnly
	{disabled}
	checked={debounce_delay_s !== undefined}
	onchange={(on) => {
		if (on) {
			debounce_delay_s = 1
		} else {
			debounce_delay_s = undefined
			debounce_key = undefined
			selectedArg = ''
			max_total_debouncing_time = undefined
			max_total_debounces_amount = undefined
		}
	}}
>
	<Label label="Delay in seconds">
		<SecondsInput {disabled} bind:seconds={debounce_delay_s} />
	</Label>
	<Label label="Custom debounce key (optional)">
		{#snippet header()}
			<Tooltip>
				Debounce keys are global. Use `$workspace` to make one workspace specific, and
				`$args[name_of_arg]` to use an argument's value.
			</Tooltip>
		{/snippet}
		<TextInput bind:value={debounce_key} inputProps={{ type: 'text', disabled, placeholder }} />
	</Label>
	<Label label="Argument to accumulate (optional)">
		{#snippet header()}
			<Tooltip>
				A list argument whose values are appended together across the debounced executions.
			</Tooltip>
		{/snippet}
		<!-- A saved argument the schema no longer has stays listed, so it can still be cleared. -->
		<Select
			items={(selectedArg && !arrayArgs.includes(selectedArg)
				? [...arrayArgs, selectedArg]
				: arrayArgs
			).map((arg) => ({ value: arg, label: arg }))}
			bind:value={() => selectedArg || undefined, (v) => (selectedArg = v ?? '')}
			placeholder="None"
			clearable
			disabled={disabled || (arrayArgs.length === 0 && !selectedArg)}
		/>
		{#if arrayArgs.length === 0}
			<div class="text-2xs text-secondary">
				No list argument in the schema. Add an argument of type "array" to accumulate it.
			</div>
		{/if}
	</Label>
	<Label label="Max total debouncing time (optional)">
		{#snippet header()}
			<Tooltip>
				The longest a run can be held back by debouncing. Past it, the run starts whatever
				debouncing is ongoing.
			</Tooltip>
		{/snippet}
		<SecondsInput {disabled} bind:seconds={max_total_debouncing_time} />
	</Label>
	<Label label="Max total debounces amount (optional)">
		{#snippet header()}
			<Tooltip>
				The most times a run can be debounced. Past it, the run starts whatever debouncing is
				ongoing.
			</Tooltip>
		{/snippet}
		<TextInput
			class="w-24"
			bind:value={max_total_debounces_amount}
			inputProps={{ type: 'number', min: 0, disabled }}
		/>
	</Label>
</SettingToggle>
