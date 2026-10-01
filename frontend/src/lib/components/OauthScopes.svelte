<script lang="ts">
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Checkbox from './common/checkbox/Checkbox.svelte'
	import { Minus, Plus } from 'lucide-svelte'

	interface Props {
		scopes?: string[]
		/** Scopes the provider is known to accept, offered as checkboxes, either a
		 * bare scope or one with a human label. Anything not in this list stays
		 * editable as free text below them. */
		options?: (string | { value: string; label: string })[]
	}

	let { scopes = $bindable(), options = [] }: Props = $props()

	let choices = $derived(
		options.map((o) =>
			typeof o == 'string' ? { value: o, label: o } : { value: o.value, label: o.label || o.value }
		)
	)
	let values = $derived(choices.map((c) => c.value))

	// Ticked options and free-text rows are kept apart from `scopes` (the only
	// value the parent binds) so a row can pass through an option's exact value
	// while typing (`…/calendar` on the way to `…/calendar.acls`) without ticking
	// or unticking anything. `lastWritten` tells a parent-side reset apart from
	// the echo of our own write.
	let ticked: string[] = $state([])
	let custom: string[] = $state([])
	let lastWritten: string | undefined = undefined

	$effect.pre(() => {
		if (!scopes) {
			scopes = []
		}
		const json = JSON.stringify([scopes, values])
		if (json != lastWritten) {
			lastWritten = json
			ticked = scopes.filter((v) => values.includes(v))
			custom = scopes.filter((v) => !values.includes(v))
		}
	})

	function write(nextTicked: string[], rows: string[]) {
		ticked = nextTicked
		custom = rows
		scopes = [...nextTicked.filter((o) => !rows.includes(o)), ...rows]
		lastWritten = JSON.stringify([scopes, values])
	}

	// Ticking an option absorbs a free-text row holding the same value. The
	// target state comes from `ticked`, not the DOM: `Checkbox` re-asserts its
	// `checked` prop on every click, so the input already reads the old value
	// again by the time `change` fires.
	function toggle(option: string, on: boolean) {
		const rest = ticked.filter((o) => o != option)
		write(on ? [...rest, option] : rest, on ? custom.filter((r) => r != option) : custom)
	}

	function setRow(i: number, value: string) {
		const rows = [...custom]
		rows[i] = value
		write(ticked, rows)
	}
</script>

{#if options.length > 0}
	<div class="flex flex-col gap-1 mb-2">
		{#each choices as { value, label } (value)}
			<label class="flex items-center gap-2 text-xs">
				<Checkbox
					checked={ticked.includes(value)}
					onChange={() => toggle(value, !ticked.includes(value))}
				/>
				{#if label == value}
					<span class="font-mono break-all">{value}</span>
				{:else}
					<span class="flex flex-col">
						<span>{label}</span>
						<span class="text-2xs text-hint font-mono break-all">{value}</span>
					</span>
				{/if}
			</label>
		{/each}
	</div>
	<span class="text-xs text-secondary">Custom scopes</span>
{/if}

{#each custom as v, i (i)}
	<div class="flex flex-row max-w-md mb-2">
		<TextInput
			value={v}
			size="sm"
			inputProps={{ oninput: (e) => setRow(i, e.currentTarget.value) }}
		/>
		<Button
			variant="default"
			unifiedSize="sm"
			btnClasses="mx-6"
			onclick={() => {
				write(
					ticked,
					custom.filter((_, j) => j != i)
				)
			}}
			startIcon={{ icon: Minus }}
			iconOnly
		/>
	</div>
{/each}

<div class="flex items-center mt-1">
	<Button
		variant="default"
		unifiedSize="sm"
		startIcon={{ icon: Plus }}
		onclick={() => {
			write(ticked, [...custom, ''])
		}}
	>
		Add item
	</Button>
	{#if custom.length > 0}
		<span class="ml-2 text-xs text-primary font-normal">
			({custom.length} item{custom.length > 1 ? 's' : ''})
		</span>
	{/if}
</div>
