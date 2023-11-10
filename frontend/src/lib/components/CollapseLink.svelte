<script lang="ts">
	import { Button } from './common'
	import { slide } from 'svelte/transition'
	import { twMerge } from 'tailwind-merge'
	import { ChevronDown } from 'lucide-svelte'

	export let open = false
	export let text: string
	export let small = false
</script>

<div class={twMerge('flex', $$props.class)}>
	<Button
		variant="border"
		color="light"
		btnClasses="text-primary {small ? 'text-xs' : ''} "
		on:click={() => (open = !open)}
		endIcon={{ icon: ChevronDown, classes: open ? 'transform rotate-180' : '' }}
	>
		{text}
	</Button>
</div>
{#if open}
	<div transition:slide|local={{ duration: 100 }}><slot /></div>
{/if}
