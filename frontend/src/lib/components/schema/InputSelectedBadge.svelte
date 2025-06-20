<script lang="ts">
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { X } from 'lucide-svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		inputSelected: 'history' | 'captures' | 'saved' | 'ai' | undefined
		labelColor?: string
		className?: string
		acceptButton?: Snippet
		onReject: () => void
	}

	let { inputSelected, className = '', acceptButton, onReject, labelColor = '' }: Props = $props()
</script>

<div class="min-h-[38px]">
	<div
		class={twMerge(
			'rounded-md flex flex-row gap-2 items-center py-1 px-2 w-fit',
			classes['info'].bgClass,
			inputSelected ? '' : 'hidden',
			className
		)}
	>
		<p class={twMerge(classes['info'].descriptionClass, 'text-xs px-2', labelColor)}>
			Using {inputSelected === 'history'
				? 'historic'
				: inputSelected === 'captures'
					? 'captures'
					: inputSelected === 'ai'
						? 'AI generated'
						: 'saved'} input arguments
		</p>
		{#if acceptButton}
			{@render acceptButton()}
		{/if}
		<Button
			color="light"
			size="xs2"
			startIcon={{ icon: X }}
			shortCut={{ key: 'esc', withoutModifier: true }}
			on:click={onReject}
		/>
	</div>
</div>
