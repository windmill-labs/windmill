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
	export let create: boolean
	export let required: boolean

	const dispatch = createEventDispatcher()

	let customItems: string[] = []

	function onCreate(newItem: string) {
		customItems = [...customItems, newItem]

		return newItem
	}
</script>

<div class="w-full flex-col">
	<div class="w-full">
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
			dropdownClassName="!text-sm !py-2 !rounded-sm !border-gray-200 !border !shadow-md !bg-surface-primary"
			className="w-full"
			noInputStyles
			onFocus={() => {
				dispatch('focus')
			}}
			{create}
			{required}
			{onCreate}
			{disabled}
			{autofocus}
			createText="Press enter to use this non-predefined value"
		/>
	</div>
	{#if value}
		<div class="-mt-2">
			<span class="text-2xs text-secondary">selected: {value}</span>
		</div>
	{/if}
</div>
