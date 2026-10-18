<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { Button } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import Select from '../select/Select.svelte'
	import { Plus, X } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'min_job_isolation_by_tag'
	const LEVELS = [
		{ label: 'Unshare', value: 'unshare' },
		{ label: 'Nsjail', value: 'nsjail' }
	]

	// `id` keeps `{#each}` from reusing a row's DOM for another row when one in the middle is removed.
	type Row = { id: number; tag: string; level: string }

	let nextId = 0
	let rows = $state<Row[]>([])
	// The JSON last read from or written to `$values[KEY]`, to tell our own writes from external
	// ones (settings load, Save, Discard) and re-hydrate only on the latter.
	let lastSynced: string | undefined = undefined

	function toRows(v: unknown): Row[] {
		if (v && typeof v === 'object' && !Array.isArray(v)) {
			return Object.entries(v as Record<string, unknown>).map(([tag, level]) => ({
				id: nextId++,
				tag,
				level: String(level)
			}))
		}
		return []
	}

	function serialize(rs: Row[]): Record<string, string> | undefined {
		// Object.fromEntries so a tag like `__proto__` becomes a real key.
		const entries = rs
			.map((r) => [r.tag?.trim(), r.level] as [string, string])
			.filter(([tag, level]) => tag && level)
		return entries.length > 0 ? Object.fromEntries(entries) : undefined
	}

	$effect(() => {
		const cur = JSON.stringify($values[KEY] ?? null)
		if (cur === lastSynced) return
		lastSynced = cur
		rows = toRows($values[KEY])
	})

	$effect(() => {
		const next = serialize(rows)
		if (lastSynced === undefined) return
		const nextStr = JSON.stringify(next ?? null)
		if (nextStr === JSON.stringify($values[KEY] ?? null)) return
		lastSynced = nextStr
		$values[KEY] = next
	})
</script>

<div class="flex flex-col gap-2">
	{#each rows as row (row.id)}
		<div class="flex items-center gap-2">
			<TextInput inputProps={{ placeholder: 'tag', disabled }} bind:value={row.tag} class="w-48" />
			<Select items={LEVELS} bind:value={row.level} {disabled} class="w-32" />
			<Button
				variant="subtle"
				unifiedSize="sm"
				iconOnly
				startIcon={{ icon: X }}
				{disabled}
				aria-label="Remove minimum"
				onclick={() => (rows = rows.filter((r) => r.id !== row.id))}
			/>
		</div>
	{/each}
	<div class="flex">
		<Button
			variant="default"
			unifiedSize="sm"
			startIcon={{ icon: Plus }}
			{disabled}
			onclick={() => (rows = [...rows, { id: nextId++, tag: '', level: 'nsjail' }])}
		>
			Add tag
		</Button>
	</div>
</div>
