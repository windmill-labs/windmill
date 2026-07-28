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
</script>

<AnimatedButton
	animate={connecting && !disableAnimation}
	baseRadius="6px"
	animationDuration="2s"
	marginWidth="2px"
>
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
		{wrapperClasses}
	>
		<Plug size={14} />
	</Button>
</AnimatedButton>
