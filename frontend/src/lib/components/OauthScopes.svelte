<script lang="ts">
	import { Button } from './common'
	import TextInput from './text_input/TextInput.svelte'
	import Checkbox from './common/checkbox/Checkbox.svelte'
	import { Minus, Plus } from 'lucide-svelte'

	interface Props {
		scopes?: string[]
		/** Scopes the provider is known to accept, offered as checkboxes. Anything
		 * not in this list stays editable as free text below them. */
		options?: string[]
	}

	let { scopes = $bindable(), options = [] }: Props = $props()

	// Free-text rows are kept apart from `scopes` (the only value the parent
	// binds) so a row survives while its text transiently equals an option, e.g.
	// typing `…/calendar` on the way to `…/calendar.acls`. `lastWritten` tells a
	// parent-side reset apart from the echo of our own write.
	let custom: string[] = $state([])
	let lastWritten: string | undefined = undefined

	$effect.pre(() => {
		if (!scopes) {
			scopes = []
		}
		const json = JSON.stringify([scopes, options])
		if (json != lastWritten) {
			lastWritten = json
			custom = scopes.filter((v) => !options.includes(v))
		}
	})

	// Options ticked by checkbox, excluding values a free-text row currently holds.
	function checkedOptions(): string[] {
		return (scopes ?? []).filter((v) => options.includes(v) && !custom.includes(v))
	}

	function write(checked: string[], rows: string[]) {
		custom = rows
		scopes = [...checked.filter((o) => !rows.includes(o)), ...rows]
		lastWritten = JSON.stringify([scopes, options])
	}

	function toggle(option: string, on: boolean) {
		const rest = checkedOptions().filter((o) => o != option)
		write(on ? [...rest, option] : rest, custom)
	}

	function setRow(i: number, value: string) {
		const rows = [...custom]
		rows[i] = value
		write(checkedOptions(), rows)
	}
</script>

{#if options.length > 0}
	<div class="flex flex-col gap-1 mb-2">
		{#each options as option (option)}
			<label class="flex items-center gap-2 text-xs">
				<Checkbox
					checked={scopes?.includes(option) ?? false}
					onChange={(e) => toggle(option, e.currentTarget.checked)}
				/>
				<span class="font-mono break-all">{option}</span>
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
					checkedOptions(),
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
			write(checkedOptions(), [...custom, ''])
		}}
	>
		Add item
	</Button>
	<span class="ml-2 text-xs text-primary font-normal">
		({custom.length} item{custom.length > 1 ? 's' : ''})
	</span>
</div>
