<script lang="ts">
	import { Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import Select from './select/Select.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import { UNIT_PRESETS, type ColorRule, type ColumnFormat } from './dbTableFormat'

	type Props = {
		column: string
		format: ColumnFormat | undefined
		onChange: (format: ColumnFormat) => void
	}
	let { column, format, onChange }: Props = $props()

	const CUSTOM = 'custom'
	// Readable in both themes: a light tint behind a dark text of the same hue.
	const RULE_PRESETS: { bg: string; text: string }[] = [
		{ bg: '#fee2e2', text: '#991b1b' },
		{ bg: '#fef3c7', text: '#92400e' },
		{ bg: '#dcfce7', text: '#166534' },
		{ bg: '#dbeafe', text: '#1e40af' }
	]

	let unit = $derived(format?.unit)
	let rules = $derived(format?.rules ?? [])
	let unitIsPreset = $derived(
		!!unit && UNIT_PRESETS.some((p) => p.symbol === unit.symbol && p.position === unit.position)
	)
	// Kept apart from the format so that picking "Custom" shows the field before anything is typed.
	let customUnit = $state(false)
	let unitChoice = $derived(
		customUnit || (unit?.symbol && !unitIsPreset) ? CUSTOM : (unit?.symbol ?? '')
	)

	function update(patch: Partial<ColumnFormat>) {
		onChange({ ...format, ...patch })
	}
	function setRule(i: number, patch: Partial<ColorRule>) {
		update({ rules: rules.map((r, j) => (j === i ? { ...r, ...patch } : r)) })
	}
</script>

<div class="flex w-80 flex-col gap-3 p-3 text-xs" data-testid="db-format-editor">
	<div class="flex flex-col gap-0.5">
		<span class="truncate font-semibold text-emphasis">Format {column}</span>
		<span class="text-2xs text-secondary">Only changes how values show in this view.</span>
	</div>

	<div class="flex flex-col gap-1">
		<span class="font-medium text-secondary">Unit</span>
		<div class="flex gap-2">
			<Select
				class="grow"
				size="sm"
				items={[
					{ label: 'None', value: '' },
					...UNIT_PRESETS.map((p) => ({
						label: p.symbol.trim(),
						value: p.symbol
					})),
					{ label: 'Custom', value: CUSTOM }
				]}
				bind:value={
					() => unitChoice,
					(v) => {
						customUnit = v === CUSTOM
						if (v === CUSTOM) update({ unit: { symbol: '', position: 'after' } })
						else update({ unit: UNIT_PRESETS.find((p) => p.symbol === v) })
					}
				}
			/>
			{#if unitChoice === CUSTOM}
				<div class="w-20 shrink-0">
					<TextInput
						size="sm"
						inputProps={{ placeholder: 'kg' }}
						bind:value={
							() => unit?.symbol ?? '',
							(v) => update({ unit: { symbol: v, position: unit?.position ?? 'after' } })
						}
					/>
				</div>
			{/if}
		</div>
		{#if unitChoice === CUSTOM}
			<ToggleButtonGroup
				bind:selected={
					() => unit?.position ?? 'after',
					(v) => update({ unit: { symbol: unit?.symbol ?? '', position: v } })
				}
			>
				{#snippet children({ item })}
					<ToggleButton value="before" label="Before" {item} small />
					<ToggleButton value="after" label="After" {item} small />
				{/snippet}
			</ToggleButtonGroup>
		{/if}
	</div>

	<div class="flex flex-col gap-1">
		<span class="font-medium text-secondary">Digits</span>
		<Select
			size="sm"
			items={[
				{ label: 'All', value: 0 },
				...Array.from({ length: 12 }, (_, i) => ({ label: String(i + 1), value: i + 1 }))
			]}
			bind:value={() => format?.digits ?? 0, (v) => update({ digits: v || undefined })}
		/>
		<span class="text-2xs text-hint">
			Significant digits. Larger numbers are shortened: 35412345 at 3 digits shows 35.4m.
		</span>
	</div>

	<div class="flex flex-col gap-1">
		<span class="font-medium text-secondary">Color rules</span>
		{#each rules as rule, i (i)}
			<div class="flex items-center gap-1.5" data-testid="db-format-rule">
				<div class="min-w-0 grow">
					<TextInput
						size="sm"
						inputProps={{ placeholder: 'e.g. >= 4 or =paid' }}
						bind:value={() => rule.condition, (v) => setRule(i, { condition: v })}
					/>
				</div>
				<input
					type="color"
					class="!h-7 !w-7 shrink-0 cursor-pointer rounded border bg-transparent !p-0.5"
					title="Background color"
					value={rule.bg ?? '#ffffff'}
					oninput={(e) => setRule(i, { bg: e.currentTarget.value })}
				/>
				<input
					type="color"
					class="!h-7 !w-7 shrink-0 cursor-pointer rounded border bg-transparent !p-0.5"
					title="Text color"
					value={rule.text ?? '#000000'}
					oninput={(e) => setRule(i, { text: e.currentTarget.value })}
				/>
				<div
					class="flex h-7 w-8 shrink-0 items-center justify-center rounded border font-medium"
					style:background-color={rule.bg}
					style:color={rule.text}
					title="Preview"
				>
					Aa
				</div>
				<Button
					variant="subtle"
					unifiedSize="xs"
					iconOnly
					startIcon={{ icon: X }}
					title="Remove rule"
					onClick={() => update({ rules: rules.filter((_, j) => j !== i) })}
				/>
			</div>
		{/each}
		<div class="flex items-center gap-1.5">
			<Button
				variant="subtle"
				unifiedSize="sm"
				startIcon={{ icon: Plus }}
				onClick={() =>
					update({
						rules: [
							...rules,
							{ condition: '', ...RULE_PRESETS[rules.length % RULE_PRESETS.length] }
						]
					})}
			>
				Add rule
			</Button>
		</div>
		<span class="text-2xs text-hint">
			Conditions use the search bar's syntax: text contains, =value is exact, numbers take &gt;,
			&gt;=, &lt;, &lt;= and !=. The first matching rule applies.
		</span>
	</div>
</div>
