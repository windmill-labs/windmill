<script lang="ts">
	import { Palette } from 'lucide-svelte'
	import Popover from '../meltComponents/Popover.svelte'
	import { NoteColor, NOTE_COLOR_SWATCHES } from './noteColors'
	import Button from '../common/button/Button.svelte'

	interface Props {
		selectedColor: NoteColor
		onColorChange: (color: NoteColor) => void
		isOpen?: boolean
	}

	let { selectedColor, onColorChange, isOpen = $bindable(false) }: Props = $props()
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
		<div class="grid grid-cols-5 gap-1" style="min-width: 140px">
			{#each Object.values(NoteColor) as color (color)}
				<button
					class="w-6 h-6 rounded-full hover:scale-110 transition-transform duration-100 {NOTE_COLOR_SWATCHES[
						color
					]} {selectedColor === color ? 'ring-2 ring-accent' : ' dark:border-gray-600'}"
					onclick={() => {
						onColorChange(color)
					}}
					title={color.charAt(0).toUpperCase() + color.slice(1)}
					aria-label={`Select ${color} color`}
				></button>
			{/each}
		</div>
	{/snippet}
</Popover>
