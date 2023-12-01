<script lang="ts">
	import AutoComplete from 'simple-svelte-autocomplete'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let disabled: boolean
	export let value: any
	export let enum_: string[] | undefined
	export let autofocus: boolean
	export let defaultValue: string | undefined
	export let valid: boolean

	const dispatch = createEventDispatcher()

	const customItems: string[] = []
</script>

<AutoComplete
	items={[...(enum_ ?? []), ...customItems]}
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
	create={true}
	onCreate={(newItem) => {
		customItems.push(newItem)
		return newItem
	}}
	{disabled}
	{autofocus}
	createText="Press enter to use this non-predefined value"
/>
