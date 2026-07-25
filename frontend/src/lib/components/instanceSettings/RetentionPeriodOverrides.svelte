<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { SecondsInput } from '../common'
	import TextInput from '../text_input/TextInput.svelte'
	import { Plus, X } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		disabled?: boolean
	}

	let { values, disabled = false }: Props = $props()

	const KEY = 'retention_period_secs_overrides'
	// Keep in sync with MAX_RETENTION_OVERRIDE_WORKSPACES in
	// backend/windmill-common/src/global_settings.rs — the backend rejects a save above this count.
	const MAX_OVERRIDES = 10

	// `id` is a stable per-row key so `{#each}` never reuses a DOM block for a different row when a
	// middle row is removed (which would re-point a focused input to another workspace's data).
	type Row = { id: number; workspace: string; seconds: number | undefined }

	let nextId = 0

	// We keep a local array (stable row identity while editing) and serialize it back to the stored
	// `{workspace_id: secs}` object, rather than mutating an object whose keys change as the operator
	// types a workspace id. `lastSynced` is the JSON we last read-from/wrote-to `$values[KEY]`; it
	// lets us tell our OWN writes apart from EXTERNAL changes (async settings load, Save, Discard,
	// reload) so we re-hydrate on the latter without a feedback loop.
	let rows = $state<Row[]>([])
	// Discrete by default: only reveal the list once there is something to show / the operator opts in.
	let expanded = $state(false)
	let lastSynced: string | undefined = undefined
	// Cap reached: the backend refuses more than MAX_OVERRIDES, so stop offering new rows here.
	let atCap = $derived(rows.length >= MAX_OVERRIDES)

	function toRows(v: unknown): Row[] {
		if (v && typeof v === 'object' && !Array.isArray(v)) {
			return Object.entries(v as Record<string, unknown>).map(([workspace, seconds]) => ({
				id: nextId++,
				workspace,
				seconds: Number(seconds)
			}))
		}
		return []
	}

	function serialize(rs: Row[]): Record<string, number> | undefined {
		// Build via Object.fromEntries (own-property semantics) rather than `obj[ws] = ...`, so a
		// workspace id like `__proto__` becomes a real key instead of hitting the prototype setter
		// and vanishing.
		const entries: [string, number][] = []
		for (const r of rs) {
			const ws = r.workspace?.trim()
			if (ws && r.seconds != undefined && r.seconds >= 0) {
				entries.push([ws, Math.round(r.seconds)])
			}
		}
		return entries.length > 0 ? Object.fromEntries(entries) : undefined
	}

	// Re-hydrate local rows from the stored value on any EXTERNAL change — the async settings load,
	// but also Save/Discard/reload, which reset `$values[KEY]` to the saved value. Skip changes we
	// caused ourselves (matches `lastSynced`), so editing doesn't re-render/lose focus.
	$effect(() => {
		const cur = JSON.stringify($values[KEY] ?? null)
		if (cur === lastSynced) return
		lastSynced = cur
		const init = toRows($values[KEY])
		rows = init
		expanded = init.length > 0
	})

	// Push local edits back into the setting value, writing only on a real change (so re-hydration
	// above doesn't ping-pong, and re-serializing the saved value never marks the form dirty).
	$effect(() => {
		const next = serialize(rows)
		// Never push before the re-hydrate effect above has read the initial value, or an empty local
		// `rows` could clobber a value that was already present at mount.
		if (lastSynced === undefined) return
		const nextStr = JSON.stringify(next ?? null)
		if (nextStr === JSON.stringify($values[KEY] ?? null)) return
		lastSynced = nextStr
		$values[KEY] = next
	})

	function addRow() {
		if (rows.length >= MAX_OVERRIDES) return
		rows = [...rows, { id: nextId++, workspace: '', seconds: undefined }]
		expanded = true
	}

	function removeRow(id: number) {
		rows = rows.filter((r) => r.id !== id)
	}
</script>

{#if !expanded}
	<button
		type="button"
		class="text-2xs text-blue-500 hover:underline disabled:text-tertiary disabled:no-underline disabled:cursor-not-allowed"
		{disabled}
		onclick={addRow}
	>
		+ Per-workspace retention override
		<span class="text-2xs text-tertiary font-semibold">EE</span>
	</button>
{:else}
	<div class="flex flex-col gap-1.5 border-l-2 border-surface-selected pl-3 mt-1">
		<span class="text-2xs text-tertiary">
			Overrides the instance retention period for specific workspaces (longer or shorter; 0 keeps
			jobs forever). Other workspaces follow the instance-wide setting.
		</span>
		{#each rows as row (row.id)}
			<!-- items-start so the workspace and period input boxes align at the top: SecondsInput is
			     taller (it has a "= N seconds" line below), which items-center would misalign against. -->
			<div class="flex items-start gap-2">
				<TextInput
					inputProps={{ placeholder: 'workspace id', disabled }}
					bind:value={row.workspace}
					class="w-40"
				/>
				<SecondsInput bind:seconds={row.seconds} {disabled} />
				<button
					type="button"
					class="text-tertiary hover:text-primary disabled:cursor-not-allowed mt-1"
					aria-label="Remove override"
					{disabled}
					onclick={() => removeRow(row.id)}
				>
					<X size={14} />
				</button>
			</div>
		{/each}
		<button
			type="button"
			class="text-2xs text-blue-500 hover:underline w-fit flex items-center gap-1 disabled:text-tertiary disabled:no-underline disabled:cursor-not-allowed"
			disabled={disabled || atCap}
			onclick={addRow}
		>
			<Plus size={12} /> Add workspace
		</button>
		{#if atCap}
			<span class="text-2xs text-tertiary">Maximum of {MAX_OVERRIDES} workspace overrides.</span>
		{/if}
	</div>
{/if}
