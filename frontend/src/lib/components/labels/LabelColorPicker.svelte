<script lang="ts">
	import type { Snippet } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ColorSwatchGrid from '$lib/components/common/colorPicker/ColorSwatchGrid.svelte'
	import type { LabelColor } from '$lib/gen'
	import { LABEL_COLORS, LABEL_COLOR_SWATCHES } from './labelColors'

	interface Props {
		color: LabelColor | undefined
		/** `undefined` clears the color and drops the label's row. */
		onSelect: (color: LabelColor | undefined) => void
		anchor: Snippet
		isOpen?: boolean
	}

	let { color, onSelect, anchor, isOpen = $bindable(false) }: Props = $props()

	function pick(next: LabelColor | undefined) {
		isOpen = false
		onSelect(next)
	}
</script>

<Popover
	placement="bottom"
	contentClasses="p-2"
	floatingConfig={{ strategy: 'absolute' }}
	usePointerDownOutside
	bind:isOpen
>
	{#snippet trigger()}
		{@render anchor()}
	{/snippet}
	{#snippet content()}
		<ColorSwatchGrid
			colors={LABEL_COLORS}
			swatches={LABEL_COLOR_SWATCHES}
			selected={color}
			onSelect={pick}
		/>
		{#if color != undefined}
			<Button
				variant="subtle"
				unifiedSize="xs"
				btnClasses="w-full mt-2"
				onclick={() => pick(undefined)}
			>
				Clear color
			</Button>
		{/if}
	{/snippet}
</Popover>
