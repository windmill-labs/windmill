<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import ColorSwatchGrid from '$lib/components/common/colorPicker/ColorSwatchGrid.svelte'
	import { sendUserToast } from '$lib/toast'
	import type { Label, LabelColor } from '$lib/gen'
	import { ArrowLeft } from 'lucide-svelte'
	import { LABEL_COLORS, LABEL_COLOR_SWATCHES } from './labelColors'
	import { setLabelColor } from './labelStore'

	interface Props {
		/** The workspace the label belongs to — not necessarily the one being navigated. */
		workspace: string | undefined
		/** The label being edited. Absent means the form creates one. */
		existing?: Label
		/** Prefills the name of a new label, e.g. with what was typed in the picker. */
		initialName?: string
		/** The name that was saved, so a caller can attach the label to the item it is editing. */
		onSaved?: (name: string) => void
		/** Renders the header with a back arrow. Absent means the surface has its own chrome. */
		onBack?: () => void
		autofocus?: boolean
	}

	let { workspace, existing, initialName = '', onSaved, onBack, autofocus }: Props = $props()

	let name = $state(existing?.name ?? initialName)
	let color: LabelColor | undefined = $state(
		// A new label saved with no colour would write nothing, so a new form starts on
		// the colour uncoloured labels already render as.
		existing ? existing.color : 'blue'
	)
	let saving = $state(false)

	let trimmedName = $derived(String(name).trim())

	async function save() {
		if (!workspace || !trimmedName) {
			return
		}
		saving = true
		try {
			await setLabelColor(workspace, trimmedName, color)
			onSaved?.(trimmedName)
		} catch (err) {
			sendUserToast(`Could not save label ${trimmedName}: ${err}`, true)
		} finally {
			saving = false
		}
	}
</script>

<div class="flex flex-col gap-3">
	{#if onBack}
		<div class="flex flex-row items-center justify-between gap-2">
			<span class="text-xs font-semibold text-emphasis">
				{existing ? 'Edit label' : 'New label'}
			</span>
			<Button
				variant="subtle"
				unifiedSize="2xs"
				startIcon={{ icon: ArrowLeft }}
				iconOnly
				aria-label="Back to labels"
				onClick={onBack}
			/>
		</div>
	{/if}
	<div class="flex flex-col gap-1">
		<span class="text-xs text-secondary">Name</span>
		<TextInput
			bind:value={name}
			size="sm"
			{autofocus}
			inputProps={{
				placeholder: 'Label name',
				// A label's name is the key of every `labels[]` entry that carries it and
				// nothing rewrites those arrays, so an existing label's name is fixed.
				disabled: existing != undefined,
				maxlength: 50,
				onkeydown: (e: KeyboardEvent) => {
					if (e.key === 'Enter') {
						e.preventDefault()
						save()
					}
				}
			}}
		/>
		{#if existing}
			<span class="text-2xs text-hint">To rename a label, edit the items that carry it.</span>
		{/if}
	</div>
	<div class="flex flex-col gap-2">
		<span class="text-xs text-secondary">Color</span>
		<ColorSwatchGrid
			colors={LABEL_COLORS}
			swatches={LABEL_COLOR_SWATCHES}
			selected={color}
			onSelect={(c) => (color = c)}
		/>
	</div>
	<div class="flex flex-row justify-between items-center gap-2">
		<Button
			variant="subtle"
			unifiedSize="sm"
			disabled={color == undefined}
			onClick={() => (color = undefined)}
		>
			Clear color
		</Button>
		<Button
			variant="accent"
			unifiedSize="sm"
			disabled={!trimmedName}
			loading={saving}
			onClick={save}
		>
			Save
		</Button>
	</div>
</div>
