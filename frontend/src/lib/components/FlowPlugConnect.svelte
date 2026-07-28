<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { Button } from './common'
	import AnimatedButton from './common/button/AnimatedButton.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		connecting: boolean
		id?: undefined | string
		wrapperClasses?: string
		/** Suppress the animated ring (e.g. the sessions modal panel). */
		disableAnimation?: boolean
		disabled?: boolean
		title?: string
	}

	let {
		connecting,
		id = undefined,
		wrapperClasses = '',
		disableAnimation = false,
		disabled = false,
		title = undefined
	}: Props = $props()

	// The ring is masked by an ::after that resolves `background: inherit` up to whatever
	// encloses the button. Give it an opaque ground while animating, or the gradient shows
	// straight through. It can't go on the wrapper itself — the scoped `.gradient-button`
	// rule outranks a utility class there.
	const animating = $derived(connecting && !disableAnimation)
</script>

<div class="flex {animating ? 'bg-surface rounded-md' : ''}">
	<AnimatedButton animate={animating} baseRadius="6px" animationDuration="2s" marginWidth="2px">
		<Button
			variant="default"
			btnClasses={twMerge(
				connecting ? 'text-accent' : '',
				'bg-surface hover:bg-surface-hover group/plug-btn overflow-clip flex p-0'
			)}
			on:click
			{disabled}
			{...title ? { title } : {}}
			{...id ? { id } : {}}
			wrapperClasses={twMerge(
				// Shrink by exactly the animated ring's margin so the button keeps its
				// footprint and nothing beside it shifts when a connect is armed.
				animating ? 'h-6 w-7' : 'h-7 w-8',
				'p-0',
				wrapperClasses
			)}
		>
			<Plug size={14} />
		</Button>
	</AnimatedButton>
</div>
