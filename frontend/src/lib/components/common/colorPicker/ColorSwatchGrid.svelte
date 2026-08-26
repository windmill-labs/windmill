<script lang="ts" generics="T extends string">
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'

	interface Props {
		colors: readonly T[]
		swatches: Record<T, string>
		/** Wider than `T`: callers hold the current color as a plain string, and one
		 * outside the palette simply rings nothing. */
		selected?: string | undefined
		onSelect: (color: T) => void
	}

	let { colors, swatches, selected, onSelect }: Props = $props()
</script>

<div class="grid grid-cols-5 gap-1" style="min-width: 140px">
	{#each colors as color (color)}
		<Button
			variant="subtle"
			unifiedSize="2xs"
			title={color.charAt(0).toUpperCase() + color.slice(1)}
			aria-label={`Select ${color} color`}
			onclick={() => onSelect(color)}
			btnClasses={twMerge(
				'w-6 h-6 p-0 rounded-full hover:scale-110 transition-transform duration-100',
				swatches[color],
				selected === color ? 'ring-2 ring-accent' : 'dark:border-gray-600'
			)}
		/>
	{/each}
</div>
