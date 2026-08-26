<script lang="ts">
	import { Palette } from 'lucide-svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import { NoteColor, NOTE_COLOR_LIST, NOTE_COLOR_SWATCHES } from './noteColors'
	import Button from '../common/button/Button.svelte'
	import ColorSwatchGrid from '../common/colorPicker/ColorSwatchGrid.svelte'

	interface Props {
		selectedColor: NoteColor
		onColorChange: (color: NoteColor) => void
		isOpen?: boolean
	}

	let {
		selectedColor,
		onColorChange,
		isOpen = $bindable(false)
	}: Props = $props()
</script>

<Popover
	placement="bottom"
	contentClasses="p-2"
	floatingConfig={{ strategy: 'absolute' }}
	usePointerDownOutside
	bind:isOpen
>
	{#snippet trigger()}
		<Button
			variant="subtle"
			unifiedSize="xs"
			selected={isOpen}
			nonCaptureEvent
			title={'Select color'}
			startIcon={{ icon: Palette }}
			iconOnly
		/>
	{/snippet}
	{#snippet content()}
		<ColorSwatchGrid
			colors={NOTE_COLOR_LIST}
			swatches={NOTE_COLOR_SWATCHES}
			selected={selectedColor}
			onSelect={onColorChange}
		/>
	{/snippet}
</Popover>
