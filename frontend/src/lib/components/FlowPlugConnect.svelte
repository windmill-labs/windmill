<script lang="ts">
	import { Plug } from 'lucide-svelte'
	import { Button } from './common'
	import AnimatedButton from './common/button/AnimatedButton.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		connecting: boolean;
		id?: undefined | string;
		wrapperClasses?: string;
	}

	let { connecting, id = undefined, wrapperClasses = '' }: Props = $props();
</script>

<AnimatedButton animate={connecting} baseRadius="6px" animationDuration="2s" marginWidth="2px">
	<Button
		variant="default"
		btnClasses={twMerge(
			connecting ? 'text-accent' : '',
			'bg-surface group/plug-btn overflow-clip flex p-0'
		)}
		on:click
		{...id ? { id } : {}}
		{wrapperClasses}
	>
		<Plug size={14} />
	</Button>
</AnimatedButton>
