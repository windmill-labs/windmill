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
		variant="border"
		color="light"
		size="xs2"
		btnClasses={twMerge(
			connecting ? 'text-blue-500' : 'text-tertiary',
			'group/plug-btn overflow-clip flex'
		)}
		on:click
		{...id ? { id } : {}}
		{wrapperClasses}
	>
		{#if !connecting}
			<span
				class="absolute -translate-x-3 opacity-0 group-hover/plug-btn:opacity-100 group-hover/plug-btn:translate-x-0 transition-all"
				>&rightarrow;</span
			>
			<Plug
				class="group-hover/plug-btn:opacity-0 group-hover/plug-btn:translate-x-3 transition-all"
				size={14}
			/>
		{:else}
			<span>&rightarrow;</span>
		{/if}
	</Button>
</AnimatedButton>
