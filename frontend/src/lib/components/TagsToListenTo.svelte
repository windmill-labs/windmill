<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { defaultTags, nativeTags } from './worker_group'
	import { safeSelectItems } from './select/utils.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { superadmin } from '$lib/stores'
	import { twMerge } from 'tailwind-merge'

	const dispatch = createEventDispatcher()
	type Props = {
		worker_tags: string[]
		customTags: string[] | undefined
		disabled?: boolean
		class?: string
	}
	let {
		worker_tags = $bindable([]),
		customTags = $bindable([]),
		disabled: _disabled = $bindable(false),
		class: clazz = ''
	}: Props = $props()

	let disabled = $derived(_disabled || !$superadmin)
</script>

<MultiSelect
	items={safeSelectItems([...(customTags ?? []), ...worker_tags, ...defaultTags, ...nativeTags])}
	bind:value={
		() => worker_tags,
		(w) => ((worker_tags = w.map((s) => s.replaceAll(' ', '_'))), dispatch('dirty'))
	}
	{disabled}
	class={twMerge(disabled ? 'border-0' : '', clazz)}
	allowClear={!disabled}
	onCreateItem={(c) => {
		worker_tags.push(c)
		dispatch('dirty')
	}}
	createText="Press Enter to use this tag"
/>
