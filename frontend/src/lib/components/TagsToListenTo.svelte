<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { defaultTags, nativeTags } from './worker_group'
	import { safeSelectItems } from './select/utils.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { superadmin } from '$lib/stores'

	const dispatch = createEventDispatcher()
	type Props = {
		worker_tags: string[]
		customTags: string[] | undefined
		disabled?: boolean
	}
	let {
		worker_tags = $bindable([]),
		customTags = $bindable([]),
		disabled: _disabled = $bindable(false)
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
	class={disabled ? 'border-0' : ''}
	allowClear={!disabled}
	onCreateItem={(c) => worker_tags.push(c)}
	createText="Press Enter to use this tag"
/>
