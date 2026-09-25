<script lang="ts">
	import { Bold, GripVertical, Italic, Plus, X } from 'lucide-svelte'
	import { Button } from './common'
	import Select from './select/Select.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import Popover from './meltComponents/Popover.svelte'
	import { RULE_PRESETS, UNIT_PRESETS, type ColorRule, type ColumnFormat } from './dbTableFormat'

	type Props = {
		column: string
		format: ColumnFormat | undefined
		onChange: (format: ColumnFormat) => void
	}
	let { column, format, onChange }: Props = $props()

	const CUSTOM = 'custom'

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
	let addRuleOpen = $state(false)
	// Which rule's preview has its presets open.
	let restyling: number | undefined = $state()

	// Order matters, a later rule overrides an earlier one: rules are dragged by their handle.
	let dragFrom: number | undefined = $state()
	let dropAt: { index: number; side: 'above' | 'below' } | undefined = $state()
	function dropRule() {
		if (dragFrom === undefined || !dropAt) return
		const moved = rules[dragFrom]
		const rest = rules.filter((_, j) => j !== dragFrom)
		let to = dropAt.index + (dropAt.side === 'below' ? 1 : 0)
		if (dragFrom < to) to--
		rest.splice(to, 0, moved)
		update({ rules: rest })
	}
</script>

{#snippet sample(style: { bg?: string; text?: string; bold?: boolean; italic?: boolean })}
	<span
		class="flex h-7 w-8 shrink-0 items-center justify-center rounded border"
		style:background-color={style.bg}
		style:color={style.text}
		style:font-weight={style.bold ? 600 : undefined}
		style:font-style={style.italic ? 'italic' : undefined}
	>
		Aa
	</span>
{/snippet}

{#snippet presets(onPick: (preset: (typeof RULE_PRESETS)[number]) => void)}
	<div class="grid grid-cols-9 gap-1 p-2">
		{#each RULE_PRESETS as preset, i (i)}
			<button
				class="rounded hover:ring-2 hover:ring-border-selected"
				title="Use these colors"
				onclick={() => onPick(preset)}
			>
				{@render sample(preset)}
			</button>
		{/each}
	</div>
{/snippet}

<div class="flex w-96 flex-col gap-3 p-3 text-xs" data-testid="db-format-editor">
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
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="relative flex items-center gap-1.5"
				data-testid="db-format-rule"
				ondragover={(e) => {
					if (dragFrom === undefined) return
					e.preventDefault()
					const r = e.currentTarget.getBoundingClientRect()
					dropAt = { index: i, side: e.clientY < r.top + r.height / 2 ? 'above' : 'below' }
				}}
				ondrop={(e) => {
					e.preventDefault()
					dropRule()
				}}
			>
				{#if dropAt?.index === i && dragFrom !== undefined}
					<div
						class="pointer-events-none absolute inset-x-0 h-0.5 rounded bg-border-selected"
						class:-top-1={dropAt.side === 'above'}
						class:-bottom-1={dropAt.side === 'below'}
					></div>
				{/if}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="-mx-1 flex h-7 shrink-0 cursor-grab items-center text-hint hover:text-primary"
					title="Drag to reorder"
					draggable="true"
					ondragstart={(e) => {
						dragFrom = i
						const row = e.currentTarget.parentElement
						if (row) e.dataTransfer?.setDragImage(row, 8, row.offsetHeight / 2)
					}}
					ondragend={() => {
						dragFrom = undefined
						dropAt = undefined
					}}
				>
					<GripVertical size={14} />
				</div>
				<div class="min-w-0 grow">
					<TextInput
						size="sm"
						inputProps={{ placeholder: 'All cells, or >= 4, =paid' }}
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
				<Button
					variant="subtle"
					unifiedSize="sm"
					iconOnly
					startIcon={{ icon: Bold }}
					selected={!!rule.bold}
					title="Bold"
					onClick={() => setRule(i, { bold: !rule.bold || undefined })}
				/>
				<Button
					variant="subtle"
					unifiedSize="sm"
					iconOnly
					startIcon={{ icon: Italic }}
					selected={!!rule.italic}
					title="Italic"
					onClick={() => setRule(i, { italic: !rule.italic || undefined })}
				/>
				<Popover
					contentClasses="z-[10001]"
					floatingConfig={{ strategy: 'fixed', placement: 'bottom-end' }}
					bind:isOpen={() => restyling === i, (open) => (restyling = open ? i : undefined)}
				>
					{#snippet trigger()}
						<span title="Pick preset colors">{@render sample(rule)}</span>
					{/snippet}
					{#snippet content()}
						{@render presets((preset) => {
							setRule(i, { bg: preset.bg, text: preset.text })
							restyling = undefined
						})}
					{/snippet}
				</Popover>
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
			<Popover
				contentClasses="z-[10001]"
				floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}
				bind:isOpen={addRuleOpen}
			>
				{#snippet trigger()}
					<Button variant="subtle" unifiedSize="sm" startIcon={{ icon: Plus }} nonCaptureEvent>
						Add rule
					</Button>
				{/snippet}
				{#snippet content()}
					{@render presets((preset) => {
						update({ rules: [...rules, { condition: '', ...preset }] })
						addRuleOpen = false
					})}
				{/snippet}
			</Popover>
		</div>
		<span class="text-2xs text-hint">
			Conditions use the search bar's syntax: text contains, =value is exact, numbers take &gt;,
			&gt;=, &lt;, &lt;= and !=. An empty condition matches every cell. Every matching rule applies,
			a later one overriding an earlier one.
		</span>
	</div>
</div>
