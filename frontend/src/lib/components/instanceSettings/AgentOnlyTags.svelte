<script lang="ts">
	import type { Writable } from 'svelte/store'
	import MultiSelect from '../select/MultiSelect.svelte'
	import { safeSelectItems } from '../select/utils.svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'agent_only_tags'

	let multiSelect = $state<MultiSelect<{ label?: string; value: any }> | undefined>(undefined)
	const searchText = $derived(multiSelect?.getFilteredInputText())

	let tags = $derived<string[]>(Array.isArray($values[KEY]) ? $values[KEY] : [])

	function setTags(next: string[]) {
		$values[KEY] = next.length > 0 ? next : undefined
	}
</script>

<MultiSelect
	bind:this={multiSelect}
	items={safeSelectItems(tags)}
	bind:value={() => tags, setTags}
	{disabled}
	allowClear={!disabled}
	placeholder="No tag"
	onCreateItem={(tag) => {
		const t = tag.trim()
		if (t && !tags.includes(t)) setTags([...tags, t])
	}}
	createText={searchText ? `Add tag: ${searchText}` : 'Add tag'}
/>
