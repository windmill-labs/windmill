<script lang="ts">
	import { Button } from '$lib/components/common'
	import TextInput from '../text_input/TextInput.svelte'
	import { Plus, Trash2 } from 'lucide-svelte'
	import { untrack } from 'svelte'

	interface Props {
		value: Record<string, string> | undefined
		onChange: (value: Record<string, string>) => void
		keyPlaceholder?: string
		valuePlaceholder?: string
		addLabel: string
		disabled?: boolean
	}

	let {
		value,
		onChange,
		keyPlaceholder = 'name',
		valuePlaceholder = 'value',
		addLabel,
		disabled = false
	}: Props = $props()

	// Rows keep entries that are still being typed (empty or duplicate keys), which a plain object
	// cannot hold; only the complete ones are written back.
	let nextId = 0
	function toRows(v: Record<string, string> | undefined) {
		return Object.entries(v ?? {}).map(([k, val]) => ({ id: nextId++, key: k, value: val }))
	}
	function fromRows() {
		return Object.fromEntries(rows.filter((r) => r.key.trim() !== '').map((r) => [r.key, r.value]))
	}
	let rows = $state(toRows(untrack(() => value)))

	// Each effect tracks one side only: edits flow out, and an outside change such as Discard
	// flows in, without either writing back what the other just wrote.
	$effect(() => {
		const next = fromRows()
		if (JSON.stringify(next) !== JSON.stringify(untrack(() => value) ?? {})) onChange(next)
	})
	$effect(() => {
		const incoming = value ?? {}
		if (JSON.stringify(incoming) !== JSON.stringify(untrack(fromRows))) rows = toRows(incoming)
	})
</script>

<div class="flex flex-col gap-2">
	{#each rows as row, i (row.id)}
		<div class="flex items-center gap-2">
			<TextInput bind:value={row.key} inputProps={{ placeholder: keyPlaceholder, disabled }} />
			<TextInput bind:value={row.value} inputProps={{ placeholder: valuePlaceholder, disabled }} />
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: Trash2 }}
				iconOnly
				{disabled}
				aria-label="Remove"
				onclick={() => rows.splice(i, 1)}
			/>
		</div>
	{/each}
	<div>
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Plus }}
			{disabled}
			onclick={() => rows.push({ id: nextId++, key: '', value: '' })}>{addLabel}</Button
		>
	</div>
</div>
