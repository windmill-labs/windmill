<script lang="ts">
	import type { EnumType } from '$lib/common'
	import { createEventDispatcher } from 'svelte'
	import Select from './Select.svelte'

	interface Props {
		disabled: boolean
		value: string | undefined
		enum_: EnumType
		autofocus: boolean | null
		defaultValue: string | undefined
		valid: boolean
		create: boolean
		enumLabels?: Record<string, string> | undefined
	}

	let {
		disabled,
		value = $bindable(),
		enum_,
		autofocus,
		defaultValue,
		valid,
		create,
		enumLabels = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let customItems: string[] = $state([])

	let items = $derived.by(() => {
		const l = [...(enum_ ? enum_ : []), ...customItems].map((item) => ({
			value: item,
			label: enumLabels?.[item] ?? item
		}))
		if (create && filterText && l.every((i) => i.value !== filterText)) {
			l.push({ value: filterText, label: `Add new: ${filterText}` })
		}
		return l
	})

	let filterText = $state('')
</script>

<div class="w-full flex-col">
	<div class="w-full">
		<Select
			inputClass={valid ? '' : '!border-red-500/60'}
			clearable
			{disabled}
			autofocus={autofocus ?? undefined}
			bind:filterText
			{items}
			bind:value={
				() => value ?? defaultValue,
				(newValue) => {
					if (newValue && items.findIndex((i) => i.value === newValue) === -1)
						customItems.push(newValue)
					value = newValue
				}
			}
			onFocus={() => dispatch('focus')}
			onBlur={() => dispatch('blur')}
		/>
	</div>
</div>
