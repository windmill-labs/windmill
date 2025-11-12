<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { Button } from './common'
	import AnimatedButton from './common/button/AnimatedButton.svelte'
	import { twMerge } from 'tailwind-merge'

	export let connecting: boolean
	export let id: undefined | string = undefined
	export let wrapperClasses = ''
</script>

<AnimatedButton animate={connecting} baseRadius="6px" animationDuration="2s" marginWidth="2px">
	<Button
		variant="default"
		btnClasses={twMerge(
			connecting ? 'text-accent' : '',
			'bg-surface hover:bg-surface-hover group/plug-btn overflow-clip flex p-0'
		)}
		on:click
		{...id ? { id } : {}}
		{wrapperClasses}
	>
		<Plug size={14} />
	</Button>
</AnimatedButton>
