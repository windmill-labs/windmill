<script lang="ts">
	import type { EnumType } from '$lib/common'
	import { createEventDispatcher } from 'svelte'
	import Select from './select/Select.svelte'

	interface Props {
		disabled: boolean
		value: string | undefined
		enum_: EnumType
		autofocus: boolean | null
		defaultValue: string | undefined
		valid: boolean
		create: boolean
		enumLabels?: Record<string, string> | undefined
		selectClass?: string
		onClear?: () => void
	}

	let {
		disabled,
		value = $bindable(),
		enum_,
		autofocus,
		defaultValue,
		valid,
		create,
		enumLabels = undefined,
		selectClass = '',
		onClear = undefined
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let customItems: string[] = $state([])

	let items = $derived.by(() => {
		const l = [...(enum_ ? enum_ : []), ...customItems]
			.map((item) => {
				if (typeof item === 'string') {
					return {
						value: item,
						label: enumLabels?.[item] ?? item
					}
				} else if (typeof item === 'object') {
					return item
				}
			})
			.filter((i) => i != undefined)
		if (create && filterText && l.every((i) => i?.value !== filterText)) {
			l.push({ value: filterText, label: `Add new: ${filterText}` })
		}
		return l
	})

	let filterText = $state('')
	let cleared = $state(false)
</script>

<div class="w-full flex-col">
	<div class="w-full">
		<Select
			error={!valid}
			clearable
			{disabled}
			autofocus={autofocus ?? undefined}
			bind:filterText
			{items}
			bind:value={
				() => value ?? (cleared ? undefined : defaultValue),
				(newValue) => {
					cleared = false
					if (newValue && items.findIndex((i) => i.value === newValue) === -1)
						customItems.push(newValue)
					value = newValue
				}
			}
			onClear={() => {
				onClear?.()
				cleared = true
				value = undefined
			}}
			onFocus={() => dispatch('focus')}
			onBlur={() => dispatch('blur')}
			inputClass={selectClass}
		/>
	</div>
</div>
