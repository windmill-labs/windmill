<script lang="ts">
	import { classes } from '$lib/components/common/alert/model'
	import { twMerge } from 'tailwind-merge'
	import Button from '$lib/components/common/button/Button.svelte'
	import { X } from 'lucide-svelte'
	import type { Snippet } from 'svelte'

	interface Props {
		/** `pending_run` is the odd one: the others report where the arguments in the form
		 * came from and undo that substitution on reject, while this one reports a chat tool
		 * call waiting on the form and declines it. */
		inputSelected: 'history' | 'captures' | 'saved' | 'ai' | 'pending_run' | undefined
		labelColor?: string
		className?: string
		acceptButton?: Snippet
		onReject: () => void
	}

	let { inputSelected, className = '', acceptButton, onReject, labelColor = '' }: Props = $props()
</script>

<!-- The reserved slot keeps the form from jumping as the badge toggles with the input
     picker's selection. A `pending_run` badge does not toggle — it stands for as long as
     the call is parked — so it holds no space open, which in a short pane is enough to
     push the form's footer out of sight. -->
<div class={inputSelected === 'pending_run' ? '' : 'min-h-[38px]'}>
	<div
		class={twMerge(
			'rounded-md flex flex-row gap-2 items-center py-1 px-2 w-fit',
			classes['info'].bgClass,
			inputSelected ? '' : 'hidden',
			className
		)}
	>
		<p class={twMerge(classes['info'].descriptionClass, 'text-xs px-2', labelColor)}>
			{#if inputSelected === 'pending_run'}
				The agent wants to run this
			{:else}
				Using {inputSelected === 'history'
					? 'historic'
					: inputSelected === 'captures'
						? 'captures'
						: inputSelected === 'ai'
							? 'AI generated'
							: 'saved'} input arguments
			{/if}
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
