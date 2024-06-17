<script lang="ts">
	import type { EnumType } from '$lib/common'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let disabled: boolean
	export let value: any
	export let enum_: EnumType
	export let autofocus: boolean | null
	export let defaultValue: string | undefined
	export let valid: boolean
	export let create: boolean
	export let required: boolean
	export let enumLabels: Record<string, string> | undefined = undefined

	$: items = enum_
		? enum_.map((item) => (enumLabels ? { value: item, label: enumLabels[item] ?? item } : item))
		: []

	const dispatch = createEventDispatcher()

	let customItems: Array<{
		value: string
		label: string
	}> = []

	function onCreate(newItem: string) {
		customItems = [
			...customItems,
			{
				value: newItem,
				label: newItem
			}
		]

		return newItem
	}
</script>

<div class="w-full flex-col">
	<div class="w-full">
		<AutoComplete
			labelFieldName={'label'}
			items={[...(required ? [] : ['']), ...items, ...customItems]}
			bind:selectedItem={value}
			inputClassName={twMerge(
				'bg-surface-secondary flex',
				valid
					? ''
					: 'border border-red-700 border-opacity-30 focus:border-red-700 focus:border-opacity-30 bg-red-100'
			)}
			value={value ?? defaultValue}
			hideArrow={true}
			dropdownClassName="!text-sm !py-2 !rounded-sm !border-gray-200 !border !shadow-md !bg-surface-primary"
			className="w-full"
			noInputStyles
			onFocus={() => {
				dispatch('focus')
			}}
			onBlur={() => {
				dispatch('blur')
			}}
			{create}
			{required}
			{onCreate}
			{disabled}
			{autofocus}
			createText="Press enter to use this non-predefined value"
		/>
	</div>
</div>
