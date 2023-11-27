<script lang="ts">
	import AutoComplete from 'simple-svelte-autocomplete'
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let customValue: boolean
	export let disabled: boolean
	export let value: any
	export let enum_: string[] | undefined
	export let autofocus: boolean
	export let defaultValue: string | undefined
	export let valid: boolean
	export let disableCustomValue: boolean = false

	const dispatch = createEventDispatcher()
</script>

<AutoComplete
	items={enum_ ?? []}
	bind:selectedItem={value}
	inputClassName={twMerge(
		'bg-surface-secondary flex',
		valid
			? ''
			: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'
	)}
	value={value ?? defaultValue}
	hideArrow={true}
	dropdownClassName="!text-sm !py-2 !rounded-sm !border-gray-200 !border !shadow-md"
	className="w-full"
	onFocus={() => {
		dispatch('focus')
	}}
	{disabled}
	{autofocus}
/>

{#if !disabled && !disableCustomValue}
	<button
		class="min-w-min !px-2 items-center text-gray-800 bg-surface-secondary border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
		on:click={() => {
			customValue = !customValue
		}}
		title="Custom Value"
	>
		<Pen class="text-tertiary" size={14} />
	</button>
{/if}
